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
    dig_t1: u16,
    dig_t2: i16,
    dig_t3: i16,
    dig_p1: u16,
    dig_p2: i16,
    dig_p3: i16,
    dig_p4: i16,
    dig_p5: i16,
    dig_p6: i16,
    dig_p7: i16,
    dig_p8: i16,
    dig_p9: i16,
}

pub struct BMP280Hardware<'a> {
    i2c: I2c<'a, esp_hal::Blocking>,
    calibration: Option<CalibrationData>,
    address: u8,
}

impl<'a> BMP280Hardware<'a> {
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

        // Default to 0x76
        let address = 0x76;

        Self {
            i2c,
            calibration: None,
            address,
        }
    }

    pub fn scan(&mut self) {
        esp_println::println!("I2C scan start");
        for addr in 0x03..=0x77 {
            if self.i2c.write(addr, &[]).is_ok() {
                esp_println::println!("Found device at 0x{:02X}", addr);
            }
        }
        esp_println::println!("I2C scan done");
    }

    pub fn read_chip_id(&mut self) -> Result<u8, &'static str> {
        let mut id = [0u8; 1];
        self.i2c
            .write_read(self.address, &[0xD0], &mut id)
            .map_err(|_| "i2c read failed")?;
        Ok(id[0])
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        let delay = Delay::new();

        // Soft reset BMP280
        self.i2c
            .write(self.address, &[0xE0, 0xB6])
            .map_err(|_| "Failed to reset sensor")?;

        delay.delay_millis(100);

        // Wait for NVM data to be copied (status bit 0 must be 0)
        for _ in 0..50 {
            let mut status = [0u8];
            if self
                .i2c
                .write_read(self.address, &[0xF3], &mut status)
                .is_ok()
            {
                if status[0] & 0x01 == 0 {
                    break;
                }
            }
            delay.delay_millis(20);
        }

        // Verify chip ID
        let chip_id = self.read_chip_id()?;
        esp_println::println!("[BMP280] Chip ID: 0x{:02X}", chip_id);

        // Read calibration data (only first 6 bytes for temperature)
        let mut calib_data = [0u8; 24];
        self.i2c
            .write_read(self.address, &[0x88], &mut calib_data)
            .map_err(|_| "Failed to read calibration data")?;

        let calibration = CalibrationData {
            dig_t1: u16::from_le_bytes([calib_data[0], calib_data[1]]),
            dig_t2: i16::from_le_bytes([calib_data[2], calib_data[3]]),
            dig_t3: i16::from_le_bytes([calib_data[4], calib_data[5]]),
            dig_p1: u16::from_le_bytes([calib_data[6], calib_data[7]]),
            dig_p2: i16::from_le_bytes([calib_data[8], calib_data[9]]),
            dig_p3: i16::from_le_bytes([calib_data[10], calib_data[11]]),
            dig_p4: i16::from_le_bytes([calib_data[12], calib_data[13]]),
            dig_p5: i16::from_le_bytes([calib_data[14], calib_data[15]]),
            dig_p6: i16::from_le_bytes([calib_data[16], calib_data[17]]),
            dig_p7: i16::from_le_bytes([calib_data[18], calib_data[19]]),
            dig_p8: i16::from_le_bytes([calib_data[20], calib_data[21]]),
            dig_p9: i16::from_le_bytes([calib_data[22], calib_data[23]]),
        };

        esp_println::println!(
            "[BMP280] Calibration: T1={}, T2={}, T3={}",
            calibration.dig_t1,
            calibration.dig_t2,
            calibration.dig_t3
        );

        self.calibration = Some(calibration);

        // BMP280 (chip ID 0x58) does NOT have humidity control register (0xF2)
        // Config register: standby 0.5ms, filter off (0xA0)
        self.i2c
            .write(self.address, &[0xF5, 0xA0])
            .map_err(|_| "Failed to configure config register")?;

        delay.delay_millis(10);

        // Control register: temp oversampling x16, pressure x16, normal mode (0x3F)
        self.i2c
            .write(self.address, &[0xF4, 0x3F])
            .map_err(|_| "Failed to configure control register")?;

        delay.delay_millis(100);

        esp_println::println!("[BMP280] Initialized - ready to measure");

        Ok(())
    }

    pub fn read_temperature(&mut self) -> Result<f32, &'static str> {
        let calib = self.calibration.ok_or("Sensor not initialized")?;

        // Read temperature data (registers 0xFA, 0xFB, 0xFC)
        let mut buf = [0u8; 3];
        self.i2c
            .write_read(self.address, &[0xFA], &mut buf)
            .map_err(|_| "I2C read error")?;

        let adc_t: i32 = ((buf[0] as i32) << 12) | ((buf[1] as i32) << 4) | ((buf[2] as i32) >> 4);

        // Bosch BMP280 datasheet compensation formula (integer version)
        let var1 = (((adc_t >> 3) - ((calib.dig_t1 as i32) << 1)) * (calib.dig_t2 as i32)) >> 11;
        let var2 = (((((adc_t >> 4) - (calib.dig_t1 as i32))
            * ((adc_t >> 4) - (calib.dig_t1 as i32)))
            >> 12)
            * (calib.dig_t3 as i32))
            >> 14;
        let t_fine = var1 + var2;
        let temperature = ((t_fine * 5 + 128) >> 8) as f32 / 100.0;

        Ok(temperature)
    }
}

pub struct SSD1306Hardware<'a> {
    pub i2c: I2c<'a, esp_hal::Blocking>,
    pub delay: Delay,
}

impl<'a> SSD1306Hardware<'a> {
    pub fn new<SDA, SCL>(i2c_periph: I2C1<'a>, sda: SDA, scl: SCL) -> Self
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

        let delay = Delay::new();

        Self { i2c, delay }
    }
}
