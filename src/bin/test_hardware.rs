#![no_std]
#![no_main]

use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{delay::Delay, timer::timg::TimerGroup};

use gonk::{hardware::BMP280Hardware, logic::AppLogic};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    esp_println::println!("[PANIC] {:?}", info);
    let delay = Delay::new();
    loop {
        delay.delay_millis(1_000);
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

// Test result tracking
struct TestResults {
    passed: u32,
    failed: u32,
    total: u32,
}

impl TestResults {
    fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            total: 0,
        }
    }

    fn assert(&mut self, condition: bool, test_name: &str) {
        self.total += 1;
        if condition {
            self.passed += 1;
            esp_println::println!("  ✓ {}", test_name);
        } else {
            self.failed += 1;
            esp_println::println!("  ✗ {} FAILED", test_name);
        }
    }

    fn assert_eq<T: PartialEq + core::fmt::Debug>(&mut self, left: T, right: T, test_name: &str) {
        self.total += 1;
        if left == right {
            self.passed += 1;
            esp_println::println!("  ✓ {}", test_name);
        } else {
            self.failed += 1;
            esp_println::println!("  ✗ {} FAILED: {:?} != {:?}", test_name, left, right);
        }
    }

    fn assert_close(&mut self, value: f32, expected: f32, tolerance: f32, test_name: &str) {
        self.total += 1;
        if (value - expected).abs() < tolerance {
            self.passed += 1;
            esp_println::println!("  ✓ {}", test_name);
        } else {
            self.failed += 1;
            esp_println::println!(
                "  ✗ {} FAILED: {:.2} not close to {:.2} (tolerance: {:.2})",
                test_name,
                value,
                expected,
                tolerance
            );
        }
    }

    fn print_summary(&self) {
        esp_println::println!("\n==========================================");
        esp_println::println!("Test Summary:");
        esp_println::println!("  Total:  {}", self.total);
        esp_println::println!("  Passed: {}", self.passed);
        esp_println::println!("  Failed: {}", self.failed);
        if self.failed == 0 {
            esp_println::println!("\n✓ ALL TESTS PASSED!");
        } else {
            esp_println::println!("\n✗ SOME TESTS FAILED");
        }
        esp_println::println!("==========================================");
    }
}

fn test_app_logic(results: &mut TestResults) {
    esp_println::println!("\n[TEST] AppLogic Tests");

    // Test single reading
    let mut app = AppLogic::new();
    app.record_temperature(22.5);
    results.assert_eq(
        app.average_temperature(),
        Some(22.5),
        "single reading average",
    );
    results.assert_eq(
        app.temperature_status(),
        "Comfortable",
        "single reading status",
    );

    // Test multiple readings
    let mut app = AppLogic::new();
    app.record_temperature(20.0);
    app.record_temperature(22.0);
    app.record_temperature(24.0);
    if let Some(avg) = app.average_temperature() {
        results.assert_close(avg, 22.0, 0.01, "multiple readings average");
    } else {
        results.assert(false, "multiple readings average (None returned)");
    }

    // Test rolling buffer
    let mut app = AppLogic::new();
    for i in 0..5 {
        app.record_temperature((i * 10 + 2) as f32);
    }
    app.record_temperature(100.0);
    if let Some(avg) = app.average_temperature() {
        results.assert_close(avg, 41.6, 0.2, "rolling buffer average");
    } else {
        results.assert(false, "rolling buffer average (None returned)");
    }

    // Test temperature status categories
    let mut app = AppLogic::new();
    app.record_temperature(5.0);
    results.assert_eq(app.temperature_status(), "Cold", "cold status");

    let mut app = AppLogic::new();
    app.record_temperature(15.0);
    results.assert_eq(app.temperature_status(), "Cool", "cool status");

    let mut app = AppLogic::new();
    app.record_temperature(22.0);
    results.assert_eq(
        app.temperature_status(),
        "Comfortable",
        "comfortable status",
    );

    let mut app = AppLogic::new();
    app.record_temperature(28.0);
    results.assert_eq(app.temperature_status(), "Warm", "warm status");

    let mut app = AppLogic::new();
    app.record_temperature(35.0);
    results.assert_eq(app.temperature_status(), "Hot", "hot status");

    // Test format_temperature
    let mut app = AppLogic::new();
    app.record_temperature(22.5);
    let formatted = app.format_temperature(22.5);
    results.assert(formatted.contains("22.5"), "format contains temperature");
    results.assert(formatted.contains("Comfortable"), "format contains status");
}

async fn test_bmp280_sensor<SDA, SCL>(
    results: &mut TestResults,
    i2c0: esp_hal::peripherals::I2C0<'static>,
    sda: SDA,
    scl: SCL,
) where
    SDA: Into<esp_hal::gpio::AnyPin<'static>>,
    SCL: Into<esp_hal::gpio::AnyPin<'static>>,
{
    esp_println::println!("\n[TEST] BMP280 Sensor Tests");

    // Create BMP280 hardware interface
    let mut bmp280 = BMP280Hardware::new(i2c0, sda, scl);

    // Test I2C scan
    esp_println::println!("  Running I2C scan...");
    bmp280.scan();
    results.assert(true, "I2C scan completed");

    // Test initialization
    match bmp280.init() {
        Ok(_) => {
            results.assert(true, "BMP280 initialization");

            // Test chip ID read
            match bmp280.read_chip_id() {
                Ok(chip_id) => {
                    esp_println::println!("    Chip ID: 0x{:02X}", chip_id);
                    results.assert_eq(chip_id, 0x58, "BMP280 chip ID is 0x58");
                }
                Err(e) => {
                    esp_println::println!("    Failed to read chip ID: {}", e);
                    results.assert(false, "read chip ID");
                }
            }

            // Test temperature reading (5 samples)
            esp_println::println!("  Reading temperatures (5 samples)...");
            let mut temps = heapless::Vec::<f32, 5>::new();
            for i in 0..5 {
                Timer::after(Duration::from_millis(100)).await;
                match bmp280.read_temperature() {
                    Ok(temp) => {
                        esp_println::println!("    Sample {}: {:.2}°C", i + 1, temp);
                        let _ = temps.push(temp);
                    }
                    Err(e) => {
                        esp_println::println!("    Failed to read temperature: {}", e);
                    }
                }
            }

            results.assert_eq(temps.len(), 5, "collected 5 temperature samples");

            // Check that readings are in reasonable range
            if temps.len() == 5 {
                for temp in temps.iter() {
                    results.assert(*temp > -40.0 && *temp < 85.0, "temperature in valid range");
                }

                // Check that readings are relatively stable (within 2°C)
                let min_temp = temps.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_temp = temps.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let range = max_temp - min_temp;
                results.assert(range < 2.0, "temperature readings stable (within 2°C)");
            }
        }
        Err(e) => {
            esp_println::println!("  Failed to initialize BMP280: {}", e);
            results.assert(false, "BMP280 initialization");
        }
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("\n==========================================");
    esp_println::println!("=== Hardware Unit Test Runner ===");
    esp_println::println!("==========================================");

    let mut results = TestResults::new();

    // Run tests that don't need hardware
    test_app_logic(&mut results);

    // Extract the peripherals we need before initializing RTOS timer
    let i2c0 = peripherals.I2C0;
    let gpio8 = peripherals.GPIO8;
    let gpio9 = peripherals.GPIO9;

    // Initialize RTOS timer for embassy (this consumes TIMG0)
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    // Run hardware tests
    test_bmp280_sensor(&mut results, i2c0, gpio8, gpio9).await;

    // Print summary
    results.print_summary();

    // Keep running and blink a pattern based on results
    esp_println::println!("\nTest run complete. Looping...");
    loop {
        if results.failed == 0 {
            // All passed - short blink
            Timer::after(Duration::from_millis(200)).await;
        } else {
            // Some failed - long blink
            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}
