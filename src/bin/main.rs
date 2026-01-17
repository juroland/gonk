#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{delay::Delay, timer::timg::TimerGroup};

use gonk::{display::init_epaper, hardware};

const HEART_BEAT_INTERVAL_MS: u64 = 5_000;

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

    loop {
        Timer::after(Duration::from_secs(5)).await;
    }
}
