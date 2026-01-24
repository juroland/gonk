//! Embassy I2C
//!
//! Depending on your target and the board you are using you have to change the
//! pins.
//!
//! This is an example of running the embassy executor with IC2. It uses an
//! LIS3DH to get accelerometer data.
//!
//! Following pins are used:
//! - SDA => GPIO4
//! - SCL => GPIO5

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3

#![no_std]
#![no_main]

use core::panic::PanicInfo;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    i2c::master::{Config, I2c},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    esp_println::println!("[PANIC] {:?}", info);
    let delay = Delay::new();
    loop {
        delay.delay_millis(1_000);
        esp_println::println!("[PANIC] continue...");
    }
}

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    let mut i2c0 = I2c::new(
        peripherals.I2C0,
        Config::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO8)
    .with_scl(peripherals.GPIO9)
    .into_async();

    // Scan
    esp_println::println!("I2C scan start");
    for address in 0x03..0x78 {
        let mut buf = [0u8; 1];
        match i2c0.write_read_async(address, &[], &mut buf).await {
            Ok(_) => {
                esp_println::println!("Found device at address 0x{:02X}", address);
            }
            Err(_) => {
                // No device at this address
                esp_println::println!("No device at address 0x{:02X}", address);
            }
        }
    }

    loop {
        Timer::after(Duration::from_millis(100)).await;
    }
}
