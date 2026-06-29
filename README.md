# urm37

[![crates.io](https://img.shields.io/crates/v/urm37.svg)](https://crates.io/crates/urm37)
[![docs.rs](https://docs.rs/urm37/badge.svg)](https://docs.rs/urm37)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

`no_std` embedded driver for the **DFRobot URM37 V5.0** ultrasonic distance sensor (SKU: SEN0001).

Supports all interface modes: synchronous/asynchronous UART, PWM trigger, and analog (ADC).

---

## Features

- `no_std` — works on any microcontroller
- Synchronous UART via [`embedded-io`](https://crates.io/crates/embedded-io)
- Asynchronous UART via [`embedded-io-async`](https://crates.io/crates/embedded-io-async) (Embassy, RTIC…)
- PWM conversion: ECHO pulse → distance in cm
- Analog conversion: raw ADC or voltage → distance in cm
- Temperature reading (UART mode)
- Internal EEPROM access (COMP threshold, auto mode, interval)
- Optional `defmt` support for embedded logging
- Zero dynamic allocation (heapless-free)

---

## Integration with STM32 & Embassy

See **[examples/STM32_EXAMPLES.md](examples/STM32_EXAMPLES.md)** for:

- 5 real-world integration patterns (UART, PWM, ADC, configuration)
- Hardware wiring diagrams for each mode
- Build and flash instructions
- Troubleshooting guide
- Performance characteristics
- Board-specific setup examples

The documentation includes patterns for:
- **Async UART** - Simple distance & temperature reading
- **PWM mode** - High-precision input capture measurements
- **Analog/ADC** - Voltage-to-distance conversion
- **EEPROM config** - Sensor threshold and mode setup
- **Production code** - Error handling, retries, statistics

---

## Wiring

| URM37 Pin | Description                              |
|-----------|------------------------------------------|
| 1 VCC     | Power supply 3.3 V – 5.5 V              |
| 2 GND     | Ground                                   |
| 3 NRST    | Reset (active low)                       |
| 4 ECHO    | PWM output (pulse width ∝ distance)      |
| 5 SERVO   | Servo control output                     |
| 6 COMP/TRIG | PWM trigger input / COMP switch output |
| 7 DAC_OUT | Analog voltage output (voltage ∝ dist.)  |
| 8 RXD     | UART RX (RS232 or TTL)                   |
| 9 TXD     | UART TX (RS232 or TTL)                   |

> WARNING: Select RS232 or TTL mode via the on-board button before wiring.
> **Never** connect a TTL MCU while the sensor is in RS232 mode — permanent damage will result.

---

## Installation

```toml
[dependencies]
# Choose the features you need:
urm37 = { version = "0.6", features = ["uart-async"] }
# or
urm37 = { version = "0.6", features = ["uart", "pwm", "analog"] }
```

---

## Usage

### Asynchronous UART (Embassy)

```rust
use urm37::uart_async::Urm37UartAsync;

let mut sensor = Urm37UartAsync::new(uart);

// Distance in centimetres
let dist_cm = sensor.read_distance().await?;

// Temperature in tenths of °C (235 = 23.5 °C)
let temp = sensor.read_temperature().await?;
let temp_c = temp as f32 / 10.0;
```

### Synchronous UART

```rust
use urm37::uart::Urm37Uart;

let mut sensor = Urm37Uart::new(uart);
let dist_cm = sensor.read_distance()?;
```

### PWM mode

```rust
use urm37::pwm::pulse_to_distance_cm;

// Trigger the measurement:
// 1. Pull COMP/TRIG low (> 1 µs)
// 2. Release high
// 3. Measure the ECHO pulse width with a timer

let pulse_us: u32 = measure_echo_us(); // your implementation
match pulse_to_distance_cm(pulse_us) {
    Some(cm) => println!("Distance: {} cm", cm),
    None     => println!("Out of range"),
}
```

### Analog mode

```rust
use urm37::analog::adc_to_distance_cm;

// 12-bit ADC (STM32, RP2040…)
let raw: u16 = adc.read(&mut dac_pin)?;
let cm = adc_to_distance_cm(raw, 4095);
```

### EEPROM configuration

```rust
use urm37::{uart_async::Urm37UartAsync, EepromRegister};

let mut sensor = Urm37UartAsync::new(uart);

// Set COMP/Switch threshold to 50 cm
sensor.set_comp_threshold(50).await?;

// Auto-measure every second (40 × 25 ms)
sensor.set_auto_mode(40).await?;

// Return to passive mode
sensor.set_passive_mode().await?;
```

---

## Cargo features

| Feature       | Default | Description                              |
|---------------|---------|------------------------------------------|
| `uart`        | no      | Synchronous UART driver (`embedded-io`)  |
| `uart-async`  | no      | Async UART driver (`embedded-io-async`)  |
| `pwm`         | no      | PWM mode utilities (`embedded-hal`)      |
| `analog`      | no      | Analog/ADC mode utilities (`embedded-hal`) |
| `defmt`       | no      | `defmt` logging on error types           |

---

## `embedded-hal` compatibility

| Crate               | Version |
|---------------------|---------|
| `embedded-hal`      | 1.0     |
| `embedded-io`       | 0.6     |
| `embedded-io-async` | 0.6     |

---

## License

Dual-licensed under MIT and Apache 2.0 — your choice.
