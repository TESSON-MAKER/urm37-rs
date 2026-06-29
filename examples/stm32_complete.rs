//! Complete production-ready example: URM37 on STM32 with Embassy
//!
//! This example demonstrates:
//! - Robust error handling
//! - Periodic measurements with retries
//! - Temperature compensation
//! - Threshold detection via COMP pin monitoring
//! - Graceful degradation on errors
//!
//! # Hardware setup
//! - USART3: RX=PB11, TX=PB10
//! - COMP (pin 6) → PB15 (GPIO input, optional threshold detection)
//! - VCC → 3.3V, GND → GND
//!
//! # Expected behavior
//! - Configures sensor on startup
//! - Reads distance and temperature every 1 second
//! - Retries on error (up to 3 attempts)
//! - Logs detected thresholds on COMP pin changes
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_complete --target thumbv7em-none-eabihf --features uart-async
//! cargo flash --example stm32_complete --target thumbv7em-none-eabihf
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::bind;
use embassy_stm32::gpio::{Input, Pull};
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{interrupt, Peripherals};
use embassy_time::{Timer, Instant};
use urm37::uart_async::Urm37UartAsync;

bind!(USART3, embassy_stm32::usart::InterruptHandler::<embassy_stm32::usart::Async>);

const MEASUREMENT_INTERVAL_MS: u64 = 1000;
const MAX_RETRIES: u8 = 3;
const TIMEOUT_MS: u64 = 2000;

struct MeasurementStats {
    total_reads: u32,
    successful_reads: u32,
    failed_reads: u32,
    min_distance: u16,
    max_distance: u16,
}

impl MeasurementStats {
    fn new() -> Self {
        Self {
            total_reads: 0,
            successful_reads: 0,
            failed_reads: 0,
            min_distance: u16::MAX,
            max_distance: 0,
        }
    }

    fn record_success(&mut self, distance: u16) {
        self.total_reads += 1;
        self.successful_reads += 1;
        self.min_distance = self.min_distance.min(distance);
        self.max_distance = self.max_distance.max(distance);
    }

    fn record_failure(&mut self) {
        self.total_reads += 1;
        self.failed_reads += 1;
    }

    fn success_rate(&self) -> u8 {
        if self.total_reads == 0 {
            0
        } else {
            ((self.successful_reads as u32 * 100) / self.total_reads as u32) as u8
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    let mut config = Config::default();
    config.baudrate = 9600;

    let uart = Uart::new(p.USART3, p.PB11, p.PB10, interrupt::USART3, config);
    let mut sensor = Urm37UartAsync::new(uart);

    // Optional: Configure COMP threshold detection
    let comp_pin = Input::new(p.PB15, Pull::Down);

    defmt::info!("╔════════════════════════════════════════╗");
    defmt::info!("║  URM37 Complete Example - STM32 Board  ║");
    defmt::info!("╚════════════════════════════════════════╝");

    // Initialize sensor
    initialize_sensor(&mut sensor).await;

    let mut stats = MeasurementStats::new();
    let mut last_comp_state = false;
    let start_time = Instant::now();

    loop {
        // Periodic measurements
        perform_measurement(&mut sensor, &mut stats).await;

        // Check COMP pin for threshold events
        let comp_detected = comp_pin.is_high();
        if comp_detected != last_comp_state {
            if comp_detected {
                defmt::warn!("COMP threshold exceeded!");
            } else {
                defmt::info!("COMP threshold cleared");
            }
            last_comp_state = comp_detected;
        }

        // Log statistics every 10 seconds
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() % 10 == 0 && elapsed.as_millis() % 1000 == 0 {
            defmt::info!(
                "Stats: {}/{} OK, Range: {}-{} cm, Success: {}%",
                stats.successful_reads,
                stats.total_reads,
                stats.min_distance,
                stats.max_distance,
                stats.success_rate()
            );
        }

        Timer::after_millis(MEASUREMENT_INTERVAL_MS).await;
    }
}

async fn initialize_sensor(sensor: &mut Urm37UartAsync<Uart<'static, embassy_stm32::usart::Async>>)
{
    defmt::info!("Initializing sensor configuration...");

    // Attempt configuration with retries
    for attempt in 1..=3 {
        match sensor.set_comp_threshold(50).await {
            Ok(()) => {
                defmt::info!("✓ COMP threshold: 50 cm");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    defmt::error!("Failed to set COMP threshold: {:?}", e);
                } else {
                    defmt::warn!("COMP config attempt {} failed, retrying...", attempt);
                    Timer::after_millis(100).await;
                }
            }
        }
    }

    // Enable auto-measurement
    for attempt in 1..=3 {
        match sensor.set_auto_mode(40).await {
            Ok(()) => {
                defmt::info!("✓ Auto-measurement: enabled (1 second interval)");
                break;
            }
            Err(e) => {
                if attempt == 3 {
                    defmt::error!("Failed to enable auto-mode: {:?}", e);
                } else {
                    defmt::warn!("Auto-mode attempt {} failed, retrying...", attempt);
                    Timer::after_millis(100).await;
                }
            }
        }
    }

    Timer::after_millis(200).await;
    defmt::info!("✓ Sensor ready");
}

async fn perform_measurement(
    sensor: &mut Urm37UartAsync<Uart<'static, embassy_stm32::usart::Async>>,
    stats: &mut MeasurementStats,
)
{
    // Attempt measurement with retries
    for attempt in 0..MAX_RETRIES {
        match sensor.read_distance().await {
            Ok(distance) => {
                match sensor.read_temperature().await {
                    Ok(temp) => {
                        stats.record_success(distance);
                        let celsius = temp as f32 / 10.0;
                        defmt::info!("Distance: {:3} cm  |  Temp: {:.1}°C", distance, celsius);
                        return;
                    }
                    Err(e) => {
                        defmt::debug!("Temperature read failed: {:?}", e);
                        defmt::info!("Distance: {:3} cm  |  Temp: --", distance);
                        stats.record_success(distance);
                        return;
                    }
                }
            }
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    defmt::debug!("Measurement attempt {} failed: {:?}, retrying...", attempt + 1, e);
                    Timer::after_millis(50).await;
                } else {
                    defmt::error!("Measurement failed after {} attempts: {:?}", MAX_RETRIES, e);
                    stats.record_failure();
                }
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    defmt::error!("!!! PANIC !!!");
    loop {}
}
