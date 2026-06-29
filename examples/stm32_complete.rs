//! Pseudo-code example: URM37 production-ready implementation
//!
//! Shows best practices for robust sensor integration.
//!
//! # Features demonstrated
//! - Automatic retry logic on failures
//! - Error handling and recovery
//! - Periodic statistics logging
//! - COMP pin threshold detection
//!
//! # Pattern
//! ```rust,no_run
//! # use urm37::uart_async::Urm37UartAsync;
//! # async fn example() {
//! const MAX_RETRIES: u8 = 3;
//!
//! // 1. Initialize sensor
//! let mut sensor = Urm37UartAsync::new(uart);
//!
//! // 2. Configure sensor parameters
//! sensor.set_comp_threshold(50).await.ok();
//! sensor.set_auto_mode(40).await.ok();
//!
//! // 3. Measurement loop with retries
//! loop {
//!     for attempt in 0..MAX_RETRIES {
//!         match sensor.read_distance().await {
//!             Ok(distance) => {
//!                 println!("Distance: {} cm", distance);
//!                 break;
//!             }
//!             Err(e) if attempt == MAX_RETRIES - 1 => {
//!                 println!("Failed after {} attempts: {:?}", MAX_RETRIES, e);
//!             }
//!             Err(_) => {
//!                 // Retry
//!             }
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! # Key features
//! - **Retries**: 3 attempts per measurement
//! - **Statistics**: Tracks success rate, min/max distance
//! - **Monitoring**: Detects COMP threshold changes
//! - **Logging**: Periodic status every 10 seconds
//! - **Graceful degradation**: Continues even on read failures
//!
//! # EEPROM Configuration
//! ```rust,no_run
//! # use urm37::uart_async::Urm37UartAsync;
//! # async fn example() {
//! # let mut sensor: Urm37UartAsync<_> = todo!();
//! // Set COMP threshold
//! sensor.set_comp_threshold(50).await?;
//!
//! // Enable auto-measurement (40 × 25ms = 1 second)
//! sensor.set_auto_mode(40).await?;
//!
//! // Return to passive mode
//! sensor.set_passive_mode().await?;
//! # Ok::<(), ()>(())
//! # }
//! ```
//!
//! # COMP Pin Monitoring
//! - Connect URM37 COMP output to GPIO input
//! - Detect when distance threshold is exceeded
//! - Useful for proximity alerts
