#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use epd_waveshare::{
    color::Color,
    epd2in13_v2::{Display2in13, Epd2in13},
    prelude::*,
};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pin},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    timer::timg::TimerGroup,
};

const SPI_FREQ_MHZ: u32 = 10;
const BUSY_TIMEOUT_MS: u32 = 10_000;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    esp_println::println!("[PANIC] {:?}", info);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

/// Wait for the e-Paper display BUSY pin to go LOW
fn wait_for_display_ready(busy: &Input, delay: &mut Delay) -> Result<(), &'static str> {
    let timeout_cycles = BUSY_TIMEOUT_MS / 10;

    for _ in 0..timeout_cycles {
        if !busy.is_high() {
            return Ok(());
        }
        delay.delay_millis(10);
    }

    Err("Display BUSY timeout")
}

/// Initialize the e-Paper display and draw initial content
fn init_epaper(
    spi: &mut ExclusiveDevice<Spi<'_, esp_hal::Blocking>, Output<'_>, Delay>,
    busy: Input<'_>,
    dc: Output<'_>,
    rst: Output<'_>,
    delay: &mut Delay,
) -> Result<(), &'static str> {
    esp_println::println!("[EPD] Initializing Waveshare 2.13\" e-Paper HAT V4");

    // Wait for display to be ready
    wait_for_display_ready(&busy, delay)?;
    esp_println::println!("[EPD] Display ready");

    // Initialize driver
    let mut epd = Epd2in13::new(spi, busy, dc, rst, delay, None)
        .map_err(|_| "Failed to create EPD driver")?;

    esp_println::println!("[EPD] Driver initialized");

    // Setup display buffer
    let mut display = Display2in13::default();
    display
        .clear(Color::White)
        .map_err(|_| "Failed to clear display")?;

    // Draw text
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Color::Black)
        .background_color(Color::White)
        .build();

    let baseline_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    Text::with_text_style(
        "Hello, ePaper!",
        Point::new(10, 10),
        text_style,
        baseline_style,
    )
    .draw(&mut display)
    .map_err(|_| "Failed to draw text")?;

    // Update display
    esp_println::println!("[EPD] Updating display...");
    epd.update_and_display_frame(spi, display.buffer(), delay)
        .map_err(|_| "Failed to update display")?;

    esp_println::println!("[EPD] Display updated successfully!");
    Ok(())
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("=== Gonk e-Paper Display Demo ===");

    // Initialize RTOS timer
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Spawn background task
    if let Err(e) = spawner.spawn(run()) {
        esp_println::println!("[ERROR] Failed to spawn task: {:?}", e);
    }

    let mut delay = Delay::new();
    let delay_for_spi = Delay::new();

    // Initialize SPI bus
    let spi_bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default().with_frequency(Rate::from_mhz(SPI_FREQ_MHZ)),
    )
    .expect("Failed to create SPI")
    .with_sck(peripherals.GPIO12)
    .with_mosi(peripherals.GPIO11);

    // Configure GPIO pins for e-Paper HAT
    let cs = Output::new(peripherals.GPIO10, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO13, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO14, Level::High, OutputConfig::default());
    let busy = Input::new(peripherals.GPIO15, InputConfig::default());

    // Create SPI device with chip select
    let mut spi =
        ExclusiveDevice::new(spi_bus, cs, delay_for_spi).expect("Failed to create SPI device");

    // Initialize and update display
    if let Err(e) = init_epaper(&mut spi, busy, dc, rst, &mut delay) {
        esp_println::println!("[ERROR] EPD initialization failed: {}", e);
    }

    // Main loop
    loop {
        Timer::after(Duration::from_secs(5)).await;
    }
}
