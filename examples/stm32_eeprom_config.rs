//! Real-world example: URM37 EEPROM configuration on STM32 with Embassy
//!
//! This example demonstrates how to:
//! 1. Configure the COMP/Switch threshold
//! 2. Enable/disable auto-measurement mode
//! 3. Set measurement intervals
//!
//! # Hardware setup
//! Same as stm32_uart_async.rs
//! - STM32 USART3: RX=PB11, TX=PB10
//! - URM37 VCC (pin 1) → 3.3V
//! - URM37 GND (pin 2) → GND
//!
//! # Features
//! - Configuration on startup (set threshold, enable auto mode)
//! - Periodic measurements
//! - Error handling with defmt logging
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_eeprom_config --target thumbv7em-none-eabihf --features uart-async
//! cargo flash --example stm32_eeprom_config --target thumbv7em-none-eabihf
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::Peripherals;
use embassy_time::{Delay, Timer};
use urm37::uart_async::Urm37UartAsync;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    // Configure USART3
    let mut config = Config::default();
    config.baudrate = 9600;

    let uart = Uart::new_blocking(
        p.USART3,
        p.PB11,
        p.PB10,
        p.DMA1_CH3,
        p.DMA1_CH2,
        config,
    );

    let mut sensor = Urm37UartAsync::new(uart);

    defmt::info!("Configuring URM37...");

    // Initialize configuration
    initialize_sensor(&mut sensor).await;

    // Main measurement loop
    loop {
        match sensor.read_distance().await {
            Ok(distance) => defmt::info!("Distance: {} cm", distance),
            Err(e) => defmt::error!("Distance read failed: {:?}", e),
        }

        match sensor.read_temperature().await {
            Ok(temp) => defmt::info!("Temperature: {}.{} °C", temp / 10, (temp % 10).abs()),
            Err(e) => defmt::error!("Temperature read failed: {:?}", e),
        }

        Timer::after_millis(500).await;
    }
}

async fn initialize_sensor<UART, E>(sensor: &mut Urm37UartAsync<UART>)
where
    UART: embassy_io_async::Read<Error = E> + embassy_io_async::Write<Error = E>,
    E: core::fmt::Debug,
{
    // Set COMP threshold to 50 cm
    match sensor.set_comp_threshold(50).await {
        Ok(()) => defmt::info!("COMP threshold set to 50 cm"),
        Err(e) => defmt::error!("Failed to set COMP threshold: {:?}", e),
    }

    // Enable auto-measurement every 500ms (20 × 25ms)
    match sensor.set_auto_mode(20).await {
        Ok(()) => defmt::info!("Auto-measurement enabled (500ms interval)"),
        Err(e) => defmt::error!("Failed to enable auto-mode: {:?}", e),
    }

    // Small delay to ensure EEPROM writes complete
    Timer::after_millis(100).await;

    defmt::info!("Sensor configuration complete");
}

// Panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
