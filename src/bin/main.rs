#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Timer};
use embedded_graphics::prelude::Point;
use esp_alloc as _;
use esp_backtrace as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, peripherals, ram, rng::Rng, timer::timg::TimerGroup};

use esp_println::{logger, println};
use esp_radio::{
    Controller,
    wifi::{
        ClientConfig, ModeConfig, ScanConfig, WifiController, WifiDevice, WifiEvent, WifiStaState,
    },
};

use gonk::display;
use gonk::hardware;
use gonk::model;

const HEART_BEAT_INTERVAL_MS: u64 = 5_000;
const REFRESH_INTERVAL_S: u64 = 60;
const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[PANIC] {:?}", info);
    let delay = Delay::new();
    loop {
        delay.delay_millis(1_000);
        println!("[PANIC] continue...");
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run_heartbeat() {
    loop {
        println!("[HEARTBEAT] System is alive");
        Timer::after(Duration::from_millis(HEART_BEAT_INTERVAL_MS)).await;
    }
}

async fn update_display<'a>(
    display: &mut display::Display<'a>,
    model: &'static embassy_sync::mutex::Mutex<
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        model::Model,
    >,
) -> Result<(), &'static str> {
    display.clear()?;

    let line_height = 10;
    let mut y = 0;

    display.draw_text("Gonk Sensor Readings", 0, 0)?;
    y += line_height;

    let start = Point {
        x: 0,
        y: y + line_height / 2,
    };
    let end = Point {
        x: 127,
        y: y + line_height / 2,
    };
    display.draw_line(start, end)?;
    y += line_height;

    {
        let m = model.lock().await;
        let temp_str: heapless::String<32> =
            heapless::format!("Temp: {:.2} C", m.temperature).unwrap();
        display.draw_text(&temp_str, 0, y)?;
        y += line_height;

        let humidity_str: heapless::String<32> =
            heapless::format!("Humidity: {:.2} %", m.humidity).unwrap();
        display.draw_text(&humidity_str, 0, y)?;
        y += line_height;

        let ip_str: heapless::String<32> = heapless::format!("IP: {}", m.ip_address).unwrap();
        display.draw_text(&ip_str, 0, y)?;
    }

    Ok(())
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_radio::wifi::sta_state() {
            WifiStaState::Connected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller.start_async().await.unwrap();
            println!("Wifi started!");

            println!("Scan");
            let scan_config = ScanConfig::default().with_max(10);
            let result = controller
                .scan_with_config_async(scan_config)
                .await
                .unwrap();
            for ap in result {
                println!("{:?}", ap);
            }
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

async fn init_wifi(
    spawner: Spawner,
    device: peripherals::WIFI<'static>,
    model: &'static embassy_sync::mutex::Mutex<
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        model::Model,
    >,
) {
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let esp_radio_ctrl = &*mk_static!(Controller<'static>, esp_radio::init().unwrap());

    let (controller, interfaces) =
        esp_radio::wifi::new(&esp_radio_ctrl, device, Default::default()).unwrap();

    let wifi_interface = interfaces.sta;

    let config = embassy_net::Config::dhcpv4(Default::default());

    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("[INFO] Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("[INFO] Got IP: {}", config.address);
            {
                let mut m = model.lock().await;
                m.ip_address = heapless::format!("{}", config.address)
                    .unwrap_or_else(|_| heapless::String::try_from("INVALID").unwrap());
            }
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

async fn update_model<'a>(
    model: &'static embassy_sync::mutex::Mutex<
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        model::Model,
    >,
    bme280: &mut hardware::BME280Hardware<'a>,
) -> Result<(), &'static str> {
    let mut m = model.lock().await;

    match bme280.read() {
        Ok(measurements) => {
            m.humidity = measurements.humidity;
            m.pressure = measurements.pressure;
            m.temperature = measurements.temperature;
        }
        Err(e) => {
            println!("[BME280] Read error: {:?}", e);
            m.humidity = -999.0;
            m.pressure = -999.0;
            m.temperature = -999.0;
        }
    }

    Ok(())
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let model = mk_static!(
        embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, model::Model>,
        embassy_sync::mutex::Mutex::new(model::Model {
            temperature: 0.0,
            pressure: 0.0,
            humidity: 0.0,
            ip_address: heapless::String::try_from("UNKNOWN").unwrap(),
        })
    );

    println!("=== Gonk ===");

    // Initialize RTOS timer for embassy
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Spawn the background heartbeat task
    if let Err(e) = spawner.spawn(run_heartbeat()) {
        println!("[ERROR] Failed to spawn task: {:?}", e);
    }

    init_wifi(spawner, peripherals.WIFI, model).await;

    // Initialize BME280 sensor
    println!("=== BME280 Temperature Sensor ===");
    let mut bme280 =
        hardware::BME280Hardware::new(peripherals.I2C0, peripherals.GPIO8, peripherals.GPIO9);

    let display_hardware =
        hardware::SSD1306Hardware::new(peripherals.I2C1, peripherals.GPIO2, peripherals.GPIO1)
            .unwrap();

    let mut display = display::Display::new(display_hardware);

    loop {
        if let Err(e) = update_model(model, &mut bme280).await {
            println!("[ERROR] Display update failed: {}", e);
        }

        if let Err(e) = update_display(&mut display, model).await {
            println!("[ERROR] Display update failed: {}", e);
        }

        Timer::after(Duration::from_secs(REFRESH_INTERVAL_S)).await;
    }
}
