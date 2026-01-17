use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::AnyPin;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig},
    peripherals::SPI2,
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
};

const SPI_FREQ_MHZ: u32 = 10;

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
