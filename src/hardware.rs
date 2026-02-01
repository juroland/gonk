use bme280::Measurements;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::AnyPin;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, I2c},
    peripherals::{I2C0, I2C1, SPI2},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
};

use bme280::i2c::BME280;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::PrimitiveStyle,
    text::{Baseline, Text, TextStyleBuilder},
};
use epd_waveshare::{
    color::Color,
    epd2in13_v2::{Display2in13, Epd2in13},
    prelude::*,
};

use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

const SPI_FREQ_MHZ: u32 = 10;

#[derive(Debug, Clone, Copy)]
pub enum DisplayType {
    EPaper,
    SSD1306,
}

pub struct DisplayHardware<'a> {
    pub spi: ExclusiveDevice<Spi<'a, esp_hal::Blocking>, Output<'a>, Delay>,
    pub busy: Input<'a>,
    pub dc: Output<'a>,
    pub rst: Output<'a>,
    pub delay: Delay,
}

impl<'a> DisplayHardware<'a> {
    pub fn new<CS, MOSI, SCK, DC, RST, BUSY>(
        spi_periph: SPI2<'a>,
        cs_gpio: CS,
        mosi_gpio: MOSI,
        sck_gpio: SCK,
        dc_gpio: DC,
        rst_gpio: RST,
        busy_gpio: BUSY,
    ) -> Self
    where
        CS: Into<AnyPin<'a>>,
        MOSI: Into<AnyPin<'a>>,
        SCK: Into<AnyPin<'a>>,
        DC: Into<AnyPin<'a>>,
        RST: Into<AnyPin<'a>>,
        BUSY: Into<AnyPin<'a>>,
    {
        let delay = Delay::new();
        let delay_for_spi = Delay::new();

        let cs_pin: AnyPin<'a> = cs_gpio.into();
        let mosi_pin: AnyPin<'a> = mosi_gpio.into();
        let sck_pin: AnyPin<'a> = sck_gpio.into();
        let dc_pin: AnyPin<'a> = dc_gpio.into();
        let rst_pin: AnyPin<'a> = rst_gpio.into();
        let busy_pin: AnyPin<'a> = busy_gpio.into();

        let spi_bus = Spi::new(
            spi_periph,
            SpiConfig::default().with_frequency(Rate::from_mhz(SPI_FREQ_MHZ)),
        )
        .unwrap()
        .with_sck(sck_pin)
        .with_mosi(mosi_pin);

        let cs = Output::new(cs_pin, Level::High, OutputConfig::default());
        let dc = Output::new(dc_pin, Level::Low, OutputConfig::default());
        let rst = Output::new(rst_pin, Level::High, OutputConfig::default());
        let busy = Input::new(busy_pin, InputConfig::default());

        let spi = ExclusiveDevice::new(spi_bus, cs, delay_for_spi).unwrap();

        Self {
            spi,
            busy,
            dc,
            rst,
            delay,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CalibrationData {
    // Temperature calibration
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    // Pressure calibration
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
    // Humidity calibration (NEW for BME280)
    dig_h1: u8,
    dig_h2: i16,
    dig_h3: u8,
    dig_h4: i16,
    dig_h5: i16,
    dig_h6: i8,
}

pub struct BME280Hardware<'a> {
    address: u8,
    bme280: BME280<I2c<'a, esp_hal::Blocking>>,
    delay: Delay,
}

impl<'a> BME280Hardware<'a> {
    pub fn new<SDA, SCL>(i2c_periph: I2C0<'a>, sda: SDA, scl: SCL) -> Self
    where
        SDA: Into<AnyPin<'a>>,
        SCL: Into<AnyPin<'a>>,
    {
        let i2c = I2c::new(
            i2c_periph,
            I2cConfig::default().with_frequency(Rate::from_khz(100)),
        )
        .unwrap()
        .with_sda(sda.into())
        .with_scl(scl.into());

        let address = 0x76;
        let mut bme280 = BME280::new_primary(i2c);

        // Initialize sensor
        let mut delay = Delay::new();
        bme280.init(&mut delay).unwrap();

        Self {
            address,
            bme280,
            delay,
        }
    }

    pub fn read(&mut self) -> Result<Measurements<esp_hal::i2c::master::Error>, bme280::Error<esp_hal::i2c::master::Error>> {
        return self.bme280.measure(&mut self.delay);
    }
}

pub struct SSD1306Hardware<'a> {
    display: Ssd1306<
        I2CInterface<I2c<'a, esp_hal::Blocking>>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
}

impl<'a> SSD1306Hardware<'a> {
    pub fn new<SDA, SCL>(i2c_periph: I2C1<'a>, sda: SDA, scl: SCL) -> Result<Self, &'static str>
    where
        SDA: Into<AnyPin<'a>>,
        SCL: Into<AnyPin<'a>>,
    {
        let i2c = I2c::new(
            i2c_periph,
            I2cConfig::default().with_frequency(Rate::from_khz(400)),
        )
        .unwrap()
        .with_sda(sda.into())
        .with_scl(scl.into());

        let interface = I2CDisplayInterface::new(i2c);

        let mut display = Ssd1306::new(
            interface,
            DisplaySize128x64,
            ssd1306::rotation::DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        display.init().map_err(|_| "Failed to initialize SSD1306")?;

        Ok(Self { display })
    }

    pub fn clear(&mut self) -> Result<(), &'static str> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(|_| "Failed to clear display")
    }

    pub fn draw_text(
        &mut self,
        text: Text<'_, MonoTextStyle<'_, BinaryColor>>,
    ) -> Result<(), &'static str> {
        text.draw(&mut self.display)
            .map_err(|_| "Failed to draw text")?;

        self.display.flush().map_err(|_| "Failed to flush display")
    }

    pub fn draw_line(
        &mut self,
        line: embedded_graphics::primitives::Line,
    ) -> Result<(), &'static str> {
        line.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut self.display)
            .map_err(|_| "Failed to draw line")?;

        self.display.flush().map_err(|_| "Failed to flush display")
    }
}
