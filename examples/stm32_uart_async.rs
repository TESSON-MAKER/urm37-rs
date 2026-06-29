//! Simple example: URM37 async UART on STM32 with Embassy
//!
//! Basic distance and temperature reading without DMA.
//!
//! # Hardware setup
//! - STM32 USART3: RX=PB11, TX=PB10
//! - URM37 RXD (pin 8) → STM32 PB11
//! - URM37 TXD (pin 9) → STM32 PB10
//! - URM37 VCC → 3.3V, GND → GND
//!
//! # Build and flash
//! ```bash
//! cargo build --example stm32_uart_async --target thumbv7em-none-eabihf --features uart-async
//! cargo flash --example stm32_uart_async --target thumbv7em-none-eabihf
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
    defmt::info!("URM37 initialized");

    loop {
        match sensor.read_distance().await {
            Ok(distance) => defmt::info!("Distance: {} cm", distance),
            Err(e) => defmt::error!("Error: {:?}", e),
        }

        match sensor.read_temperature().await {
            Ok(temp) => defmt::info!("Temperature: {}.{} °C", temp / 10, temp % 10),
            Err(e) => defmt::error!("Error: {:?}", e),
        }

        Timer::after_millis(500).await;
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
