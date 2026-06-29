//! Pseudo-code example: URM37 async UART on STM32 with Embassy
//!
//! This is an illustrative example showing the pattern.
//! Adapt to your specific STM32 board and embassy-stm32 version.
//!
//! # Hardware setup
//! - STM32 USART: Configure for your board
//! - URM37 TXD → MCU RX
//! - URM37 RXD → MCU TX
//! - VCC → 3.3V, GND → GND
//!
//! # Pattern
//! ```rust,no_run
//! # use urm37::uart_async::Urm37UartAsync;
//! # async fn example() {
//! // 1. Initialize UART (embassy-stm32)
//! let uart = /* your UART setup */;
//!
//! // 2. Create sensor driver
//! let mut sensor = Urm37UartAsync::new(uart);
//!
//! // 3. Read distance
//! match sensor.read_distance().await {
//!     Ok(cm) => println!("Distance: {} cm", cm),
//!     Err(e) => println!("Error: {:?}", e),
//! }
//!
//! // 4. Read temperature (tenths of °C)
//! match sensor.read_temperature().await {
//!     Ok(temp) => println!("Temperature: {}.{} °C", temp / 10, temp % 10),
//!     Err(e) => println!("Error: {:?}", e),
//! }
//! # }
//! ```
//!
//! # Full working example (STM32L4 with Embassy 0.6)
//! ```
//! cargo build --example stm32_uart_async --target thumbv7em-none-eabihf --features uart-async
//! ```
//! (Note: Requires board support package and memory.x configuration)

fn main() {
    println!("This is a pseudo-code example.");
    println!("See the documentation above for the async UART pattern.");
    println!("Adapt to your STM32 board: https://docs.embassy.dev/");
}
