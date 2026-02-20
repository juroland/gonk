#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    timer::timg::TimerGroup,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task(pool_size = 2)]
async fn button_watcher(mut button: Input<'static>, tag: &'static str) {
    esp_println::println!("Watching for button '{}' presses...", tag);

    loop {
        button.wait_for_falling_edge().await;

        // Debounce delay - wait for button to stabilize
        Timer::after(Duration::from_millis(50)).await;

        if button.is_low() {
            esp_println::println!("Button '{}' was pressed!", tag);

            // Wait for button release to avoid multiple triggers
            button.wait_for_rising_edge().await;

            esp_println::println!("Button '{}' was released!", tag);

            // Debounce the release
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let config = InputConfig::default().with_pull(Pull::Up);

    let green_button = Input::new(peripherals.GPIO12, config);
    let blue_button = Input::new(peripherals.GPIO13, config);

    spawner
        .spawn(button_watcher(green_button, "green"))
        .unwrap();
    spawner.spawn(button_watcher(blue_button, "blue")).unwrap();

    loop {
        esp_println::println!("Main loop working...");

        Timer::after(Duration::from_millis(1000)).await;
    }
}
