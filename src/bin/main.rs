#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{delay::Delay, timer::timg::TimerGroup};

use gonk::{
    display::{init_epaper, init_ssd1306},
    hardware::{self, DisplayType},
};

const HEART_BEAT_INTERVAL_MS: u64 = 5_000;

// Choose your display type here
// DisplayType::EPaper - Uses SPI on GPIO10(CS), GPIO11(MOSI), GPIO12(SCK), GPIO13(DC), GPIO14(RST), GPIO15(BUSY)
// DisplayType::SSD1306 - Uses I2C on GPIO2(SDA), GPIO1(SCL), address 0x3C
const DISPLAY_TYPE: DisplayType = DisplayType::SSD1306; // or DisplayType::EPaper

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    esp_println::println!("[PANIC] {:?}", info);
    let delay = Delay::new();
    loop {
        delay.delay_millis(1_000);
        esp_println::println!("[PANIC] continue...");
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run_heartbeat() {
    loop {
        esp_println::println!("[HEARTBEAT] System is alive");
        Timer::after(Duration::from_millis(HEART_BEAT_INTERVAL_MS)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("=== Gonk ===");

    // Initialize RTOS timer for embassy
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Spawn the background heartbeat task
    if let Err(e) = spawner.spawn(run_heartbeat()) {
        esp_println::println!("[ERROR] Failed to spawn task: {:?}", e);
    }

    // Initialize BMP280 sensor
    esp_println::println!("=== BMP280 Temperature Sensor ===");
    let mut bmp280 =
        hardware::BMP280Hardware::new(peripherals.I2C0, peripherals.GPIO8, peripherals.GPIO9);

    if let Err(e) = bmp280.init() {
        esp_println::println!("[ERROR] BMP280 init failed: {}", e);
        loop {
            Timer::after(Duration::from_secs(1)).await;
        }
    }

    // Initialize display based on chosen type
    match DISPLAY_TYPE {
        DisplayType::EPaper => {
            let display = hardware::DisplayHardware::new(
                peripherals.SPI2,
                peripherals.GPIO10,
                peripherals.GPIO11,
                peripherals.GPIO12,
                peripherals.GPIO13,
                peripherals.GPIO14,
                peripherals.GPIO15,
            );
            let _ = init_epaper(display);
        }
        DisplayType::SSD1306 => {
            let display = hardware::SSD1306Hardware::new(
                peripherals.I2C1,
                peripherals.GPIO2,
                peripherals.GPIO1,
            );
            let _ = init_ssd1306(display);
        }
    }

    loop {
        match bmp280.read_temperature() {
            Ok(temp) => esp_println::println!("[BMP280] Temperature: {:.2}°C", temp),
            Err(e) => esp_println::println!("[BMP280] Read error: {}", e),
        }

        Timer::after(Duration::from_secs(2)).await;
    }
}
