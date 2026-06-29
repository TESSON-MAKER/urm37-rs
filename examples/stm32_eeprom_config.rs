//! Example: URM37 EEPROM configuration on STM32 with Embassy
//!
//! Configure sensor parameters:
//! - COMP threshold
//! - Auto-measurement mode
//! - Measurement intervals
//!
//! # Hardware setup
//! - STM32 USART3: RX=PB11, TX=PB10
//! - URM37 VCC → 3.3V, GND → GND
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_eeprom_config --target thumbv7em-none-eabihf --features uart-async
//! cargo flash --example stm32_eeprom_config --target thumbv7em-none-eabihf
//! ```

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::bind;
use embassy_stm32::usart::{Config, Uart};
use embassy_stm32::{interrupt, Peripherals};
use embassy_time::Timer;
use urm37::uart_async::Urm37UartAsync;

bind!(USART3, embassy_stm32::usart::InterruptHandler::<embassy_stm32::usart::Async>);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = Peripherals::take();

    let mut config = Config::default();
    config.baudrate = 9600;

    let uart = Uart::new(p.USART3, p.PB11, p.PB10, interrupt::USART3, config);
    let mut sensor = Urm37UartAsync::new(uart);

    defmt::info!("Configuring URM37");

    // Set COMP threshold to 50 cm
    if let Err(e) = sensor.set_comp_threshold(50).await {
        defmt::error!("COMP config failed: {:?}", e);
    } else {
        defmt::info!("✓ COMP threshold: 50 cm");
    }

    // Enable auto-measurement every 1 second (40 × 25ms)
    if let Err(e) = sensor.set_auto_mode(40).await {
        defmt::error!("Auto mode failed: {:?}", e);
    } else {
        defmt::info!("✓ Auto-measurement: 1 second interval");
    }

    Timer::after_millis(100).await;
    defmt::info!("Configuration complete");

    // Periodic measurements
    loop {
        match sensor.read_distance().await {
            Ok(d) => defmt::info!("Distance: {} cm", d),
            Err(e) => defmt::error!("Error: {:?}", e),
        }

        Timer::after_millis(500).await;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
