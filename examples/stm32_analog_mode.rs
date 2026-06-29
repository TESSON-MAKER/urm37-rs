//! Pseudo-code example: URM37 analog mode on STM32
//!
//! Shows how to read distance via analog DAC_OUT pin.
//!
//! # Hardware setup
//! - DAC_OUT (pin 7) → ADC input
//! - VCC → 3.3V or 5V
//! - GND → GND
//!
//! # Pattern
//! ```rust,no_run
//! # use urm37::analog::adc_to_distance_cm;
//! # async fn example() {
//! // 1. Configure ADC (embassy-stm32)
//! let mut adc = /* ADC setup */;
//!
//! // 2. Read ADC channel
//! let raw: u16 = adc.read(/* DAC_OUT pin */).await;
//!
//! // 3. Convert to distance
//! // 12-bit ADC: max = 4095
//! // 10-bit ADC: max = 1023
//! let distance = adc_to_distance_cm(raw, 4095);
//!
//! println!("Distance: {} cm", distance);
//!
//! // 4. Optional: Calculate voltage
//! // 3.3V supply
//! let voltage_mv = (raw as u32 * 3300) / 4095;
//! println!("Voltage: {} mV", voltage_mv);
//! # }
//! ```
//!
//! # Resolution
//! - 12-bit ADC @ 3.3V: 800cm / 4095 = 0.2 cm/LSB (~2cm per 10 LSBs)
//! - 10-bit ADC @ 5.0V: 800cm / 1023 = 0.78 cm/LSB
//!
//! # Advantages
//! - No UART latency
//! - Simplest wiring
//! - Single ADC input
//! - Linear 0V→Vcc mapping

fn main() {
    println!("This is a pseudo-code example.");
    println!("See the documentation above for the analog ADC pattern.");
    println!("Adapt to your STM32 board: https://docs.embassy.dev/");
}
