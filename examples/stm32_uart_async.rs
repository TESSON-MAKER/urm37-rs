//! Real-world example: URM37 async UART on STM32 with Embassy
//!
//! This example shows how to use the URM37 driver with an STM32 microcontroller
//! and the Embassy async runtime.
//!
//! # Hardware setup
//! - STM32 USART3: RX=PB11, TX=PB10
//! - URM37 RXD (pin 8) → STM32 PB11 (USART3_RX)
//! - URM37 TXD (pin 9) → STM32 PB10 (USART3_TX)
//! - URM37 VCC (pin 1) → 3.3V (or 5V with level shifter)
//! - URM37 GND (pin 2) → GND
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_uart_async --target thumbv7em-none-eabihf --features uart-async
//! cargo flash --example stm32_uart_async --target thumbv7em-none-eabihf
//! ```
//!
//! # Expected output (via RTT/defmt)
//! ```
//! Distance: 123 cm
//! Temperature: 24.5 °C
//! COMP Threshold: 50 cm
//! Distance: 125 cm
//! ...
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::Speed;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::Peripherals;
use embassy_time::{Delay, Timer};
use urm37::uart_async::Urm37UartAsync;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    // Configure USART3 on PB10 (TX) and PB11 (RX)
    let mut config = Config::default();
    config.baudrate = 9600;

    let uart = Uart::new_blocking(
        p.USART3,
        p.PB11, // RX
        p.PB10, // TX
        p.DMA1_CH3,
        p.DMA1_CH2,
        config,
    );

    // Create URM37 sensor driver
    let mut sensor = Urm37UartAsync::new(uart);

    defmt::info!("URM37 sensor initialized");

    loop {
        // Read distance measurement
        match sensor.read_distance().await {
            Ok(distance) => defmt::info!("Distance: {} cm", distance),
            Err(e) => defmt::error!("Distance read error: {:?}", e),
        }

        // Read temperature
        match sensor.read_temperature().await {
            Ok(temp) => {
                let celsius = temp as f32 / 10.0;
                defmt::info!("Temperature: {} °C", temp as f32 / 10.0);
            }
            Err(e) => defmt::error!("Temperature read error: {:?}", e),
        }

        // Wait 500ms before next measurement
        Timer::after_millis(500).await;
    }
}

// Panic handler for no_std environment
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
