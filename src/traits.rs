//! Hardware abstraction traits

/// Trait for temperature sensors
pub trait TemperatureSensor {
    /// Initialize the sensor
    fn init(&mut self) -> Result<(), &'static str>;
    
    /// Read temperature in Celsius
    fn read_temperature(&mut self) -> Result<f32, &'static str>;
}

/// Trait for display devices
pub trait Display {
    /// Initialize the display
    fn init(&mut self) -> Result<(), &'static str>;
    
    /// Clear the display
    fn clear(&mut self) -> Result<(), &'static str>;
    
    /// Draw text at specified position
    fn draw_text(&mut self, text: &str, x: i32, y: i32) -> Result<(), &'static str>;
    
    /// Update/flush the display (show the buffer)
    fn update(&mut self) -> Result<(), &'static str>;
}

/// Trait for I2C operations
pub trait I2cBus {
    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), &'static str>;
    fn write_read(&mut self, addr: u8, write: &[u8], read: &mut [u8]) -> Result<(), &'static str>;
}
