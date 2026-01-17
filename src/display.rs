use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};
use epd_waveshare::{
    color::Color,
    epd2in13_v2::{Display2in13, Epd2in13},
    prelude::*,
};

use crate::hardware::DisplayHardware;

const BUSY_TIMEOUT_MS: u32 = 10_000;

/// Wait for the e-Paper display BUSY pin to go LOW
fn wait_for_display_ready(hw: &DisplayHardware) -> Result<(), &'static str> {
    let timeout_cycles = BUSY_TIMEOUT_MS / 10;

    for _ in 0..timeout_cycles {
        if !hw.busy.is_high() {
            return Ok(());
        }
        hw.delay.delay_millis(10);
    }

    Err("Display BUSY timeout")
}

/// Initialize the e-Paper display and draw initial content
pub fn init_epaper(mut hw: DisplayHardware) -> Result<(), &'static str> {
    esp_println::println!("[EPD] Initializing Waveshare 2.13\" e-Paper HAT V4");

    // Wait for display to be ready
    wait_for_display_ready(&hw)?;
    esp_println::println!("[EPD] Display ready");

    // Initialize driver
    let mut epd = Epd2in13::new(&mut hw.spi, hw.busy, hw.dc, hw.rst, &mut hw.delay, None)
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
    epd.update_and_display_frame(&mut hw.spi, display.buffer(), &mut hw.delay)
        .map_err(|_| "Failed to update display")?;

    esp_println::println!("[EPD] Display updated successfully!");
    Ok(())
}
