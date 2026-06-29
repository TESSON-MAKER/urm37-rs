//! PWM trigger mode example for URM37 sensor.
//!
//! This example demonstrates how to use the URM37 driver in PWM trigger mode.
//! The driver manages the TRIG pin, and you provide the pulse measurement via a closure.
//!
//! # Hardware setup
//! - TRIG (pin 6, COMP) → GPIO output
//! - ECHO (pin 4) → GPIO input with timer/input capture
//! - VCC (pin 1) → 3.3V or 5V
//! - GND (pin 2) → GND
//!
//! # Typical measurement loop
//! ```ignore
//! let mut sensor = Urm37Pwm::new(trig_pin)?;
//!
//! loop {
//!     let distance = sensor.measure(&mut delay, || async {
//!         // Use your HAL's input capture or timer to measure pulse width
//!         timer.measure_pulse_us().await
//!     }).await?;
//!
//!     match distance {
//!         Some(cm) => println!("Distance: {} cm", cm),
//!         None => println!("Out of range"),
//!     }
//! }
//! ```

use urm37::pwm::{
    distance_cm_to_pulse_us, pulse_to_distance_cm, MAX_VALID_PULSE_US, US_PER_CM,
};

fn main() {
    println!("URM37 PWM Trigger Mode Example");
    println!("==============================\n");

    println!("Hardware Requirements:");
    println!("  - GPIO output for TRIG pin");
    println!("  - Timer/input capture for ECHO pulse measurement");
    println!("  - ~1-3 µs pulse on TRIG triggers measurement\n");

    println!("Key Constants:");
    println!("  - US_PER_CM: {} µs/cm", US_PER_CM);
    println!("  - MAX_VALID_PULSE_US: {} µs ({} cm)", MAX_VALID_PULSE_US, MAX_VALID_PULSE_US / US_PER_CM);
    println!();

    // Conversion examples
    println!("Conversion Examples:");
    println!("--------------------");

    // Example 1: 100 cm → pulse width
    let distance = 100u16;
    let pulse = distance_cm_to_pulse_us(distance);
    println!("{}  cm → {} µs pulse", distance, pulse);

    // Example 2: Different distances
    for dist in [10, 50, 100, 200, 400, 800].iter() {
        let pulse = distance_cm_to_pulse_us(*dist);
        println!("{}  cm → {} µs", dist, pulse);
    }

    println!();

    // Example 3: Pulse width → distance
    println!("Pulse to Distance Conversion:");
    println!("---------------------------");

    let test_pulses = [500, 2500, 5000, 10000, 20000, 40000];
    for pulse in test_pulses.iter() {
        match pulse_to_distance_cm(*pulse) {
            Some(cm) => println!("{:5} µs → {:3} cm", pulse, cm),
            None => println!("{:5} µs → out of range", pulse),
        }
    }

    println!();
    println!("Special cases:");
    println!("  0 µs    → {:?}", pulse_to_distance_cm(0));
    println!("  50000 µs (sentinel) → {:?}", pulse_to_distance_cm(50000));
    println!("  40001 µs (max+1) → {:?}", pulse_to_distance_cm(40001));
}
