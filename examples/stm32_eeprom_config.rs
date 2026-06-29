//! Pseudo-code example: URM37 EEPROM configuration on STM32
//!
//! Shows how to configure sensor parameters.
//!
//! # Pattern
//! ```rust,no_run
//! # use urm37::uart_async::Urm37UartAsync;
//! # async fn example() {
//! // Initialize UART and sensor
//! let uart = /* your UART setup */;
//! let mut sensor = Urm37UartAsync::new(uart);
//!
//! // Set COMP threshold to 50 cm
//! sensor.set_comp_threshold(50).await.ok();
//!
//! // Enable auto-measurement every 1 second (40 × 25ms)
//! sensor.set_auto_mode(40).await.ok();
//!
//! // Switch back to passive mode if needed
//! sensor.set_passive_mode().await.ok();
//!
//! // Periodic measurements
//! loop {
//!     match sensor.read_distance().await {
//!         Ok(cm) => println!("Distance: {} cm", cm),
//!         Err(e) => println!("Error: {:?}", e),
//!     }
//! }
//! # }
//! ```
//!
//! # Configuration options
//! - `set_comp_threshold(distance_cm)` - Set COMP/Switch distance threshold
//! - `set_auto_mode(interval)` - Enable auto-measurement (interval in units of 25ms)
//! - `set_passive_mode()` - Return to on-demand measurement mode
//! - `eeprom_read/write()` - Direct EEPROM register access

fn main() {
    println!("This is a pseudo-code example.");
    println!("See the documentation above for the EEPROM configuration pattern.");
    println!("Adapt to your STM32 board: https://docs.embassy.dev/");
}
