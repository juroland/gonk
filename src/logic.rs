//! Business logic layer (hardware-independent)

use crate::traits::{Display, TemperatureSensor};
use core::fmt::Write;

/// Application state for testable business logic
pub struct AppLogic {
    temperature_readings: [Option<f32>; 5],
    reading_index: usize,
}

impl AppLogic {
    pub fn new() -> Self {
        Self {
            temperature_readings: [None; 5],
            reading_index: 0,
        }
    }

    /// Record a temperature reading
    pub fn record_temperature(&mut self, temp: f32) {
        self.temperature_readings[self.reading_index] = Some(temp);
        self.reading_index = (self.reading_index + 1) % self.temperature_readings.len();
    }

    /// Calculate average temperature from recorded readings
    pub fn average_temperature(&self) -> Option<f32> {
        let mut sum = 0.0;
        let mut count = 0;

        for reading in self.temperature_readings.iter() {
            if let Some(temp) = reading {
                sum += temp;
                count += 1;
            }
        }

        if count == 0 {
            None
        } else {
            Some(sum / count as f32)
        }
    }

    /// Get temperature status message
    pub fn temperature_status(&self) -> &'static str {
        match self.average_temperature() {
            Some(temp) if temp < 10.0 => "Cold",
            Some(temp) if temp < 20.0 => "Cool",
            Some(temp) if temp < 25.0 => "Comfortable",
            Some(temp) if temp < 30.0 => "Warm",
            Some(_) => "Hot",
            None => "No data",
        }
    }

    /// Format temperature reading for display
    pub fn format_temperature(&self, temp: f32) -> heapless::String<32> {
        let mut buffer = heapless::String::new();
        let _ = write!(buffer, "{:.1}C {}", temp, self.temperature_status());
        buffer
    }
}

/// Update display with sensor reading
pub fn update_display_with_sensor<D: Display, T: TemperatureSensor>(
    display: &mut D,
    sensor: &mut T,
    app: &mut AppLogic,
) -> Result<(), &'static str> {
    // Read temperature
    let temp = sensor.read_temperature()?;
    app.record_temperature(temp);

    // Update display
    display.clear()?;

    let temp_str = app.format_temperature(temp);
    display.draw_text(temp_str.as_str(), 10, 10)?;

    if let Some(avg) = app.average_temperature() {
        let mut avg_str = heapless::String::<32>::new();
        let _ = write!(avg_str, "Avg: {:.1}C", avg);
        display.draw_text(avg_str.as_str(), 10, 25)?;
    }

    display.update()?;
    Ok(())
}
