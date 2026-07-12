# URM37 Usage Examples

Complete examples for all three URM37 sensor modes with Embassy + STM32F767ZI.

## 1. UART Async Mode

**Best for:** Distance + Temperature reading, EEPROM configuration, passive mode (sensor waits for UART command).

```rust
use urm37::uart_async::Urm37UartAsync;
use embedded_io_async::{Read, Write};

// Setup UART (adjust pins/peripheral for your board)
let mut sensor = Urm37UartAsync::new(uart_wrapper);

loop {
    // Read distance
    match sensor.read_distance().await {
        Ok(distance) => println!("Distance: {} cm", distance),
        Err(e) => println!("Error: {:?}", e),
    }

    // Read temperature
    match sensor.read_temperature().await {
        Ok(temp) => println!("Temp: {:.1}°C", temp),
        Err(e) => println!("Error: {:?}", e),
    }

    // Configure thresholds (optional)
    sensor.set_comp_threshold(50).await?;
}
```

**Hardware:**
- UART5: RX=PD2, TX=PC12 (adjust for your board)
- URM37: GND=GND, 5V=5V, TX→RX, RX→TX

**Key methods:**
- `read_distance()` → `Result<u16, Error>`
- `read_temperature()` → `Result<f32, Error>`
- `eeprom_read(register)` → `Result<u8, Error>`
- `eeprom_write(register, value)` → `Result<(), Error>`
- `set_comp_threshold(distance)` → `Result<(), Error>`

---

## 2. Analog ADC Mode

**Best for:** Simple, fast distance reading via voltage output. No UART needed.

```rust
use urm37::analog::adc_to_distance_cm;
use embassy_stm32::adc::{Adc, SampleTime};

let mut adc = Adc::new(p.ADC1);
let mut pin = p.PA4;

loop {
    let raw: u16 = adc.blocking_read(&mut pin, SampleTime::CYCLES112);

    match adc_to_distance_cm(raw, 4095) {
        Some(cm) => println!("Distance: {} cm", cm),
        None => println!("Out of range"),
    }

    Timer::after_millis(100).await;
}
```

**Hardware:**
- ADC1: PA4 (analog voltage from URM37)
- URM37: GND=GND, 5V=5V, ANALOG=PA4

**Conversion formula:**
- ADC reading range: 0-4095 (12-bit)
- Distance = (raw / 4095) × (VCC / 0.006) volts
- Simplified: ~2 cm per LSB at 3.3V VCC

---

## 3. PWM Mode (Automatic)

**Best for:** High precision µs-resolution timing, autonomous/continuous measurement.

### Prerequisites

Configure sensor in **autonomous mode** via UART:
```rust
sensor.eeprom_write(EepromRegister::MeasureMode, 0xAA).await?;  // Autonomous
```

### Implementation

```rust
use urm37::pwm::{Urm37Pwm, PulseReader};
use embedded_hal_async::delay::DelayNs;
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::Channel;

// Implement PulseReader for echo measurement
struct MyPulseReader<'d> {
    ic: InputCapture<'d, TIM2>,
}

impl<'d> PulseReader for MyPulseReader<'d> {
    async fn measure_pulse(&mut self) -> Option<u32> {
        // Synchronize on rising edge (idle state)
        self.ic.wait_for_rising_edge(Channel::Ch1).await;

        // Measure LOW pulse: HIGH → LOW → HIGH
        let t_fall = self.ic.wait_for_falling_edge(Channel::Ch1).await;
        let t_rise = self.ic.wait_for_rising_edge(Channel::Ch1).await;

        let duration_us = t_rise.wrapping_sub(t_fall);

        if duration_us > 0 && duration_us < 50000 {
            Some(duration_us)
        } else {
            None
        }
    }
}

// Implement DelayNs for timing
struct MyDelay;

impl DelayNs for MyDelay {
    async fn delay_ms(&mut self, ms: u32) {
        if ms > 0 {
            Timer::after_millis(ms as u64).await;
        }
    }

    // ... implement delay_us, delay_ns similarly
}

// Use PWM driver
let trig = Output::new(p.PA0, Level::High, Speed::Low);
let ic = InputCapture::new(p.TIM2, Some(CapturePin::new(p.PA5, Pull::None)), /* ... */);
let mut sensor = Urm37Pwm::new(trig, MyPulseReader { ic }, MyDelay)?;

loop {
    match sensor.read_distance().await {
        Ok(Some(cm)) => println!("Distance: {} cm", cm),
        Ok(None) => println!("Out of range"),
        Err(e) => println!("Error: {:?}", e),
    }
    Timer::after_millis(100).await;
}
```

**Hardware (Autonomous mode):**
- GPIO PA0: TRIG output (optional, sensor self-triggers)
- TIM2 CH1 PA5: ECHO input (InputCapture at 1 MHz)
- URM37: GND=GND, 5V=5V, ECHO=PA5

**Pulse measurement:**
- ECHO line is normally HIGH
- Measurement pulse: ECHO goes LOW for (distance_cm × 50) µs
- Example: 100 cm → 5000 µs LOW pulse

---

## Mode Comparison

| Feature | UART | Analog | PWM |
|---------|------|--------|-----|
| **Precision** | ±1 cm | ~2 cm/LSB | µs (0.5 cm) |
| **Speed** | ~50 ms per reading | ~100 µs | ~10 ms (async) |
| **Wiring** | TX/RX only | 1 analog pin | ECHO pin + TRIG |
| **Features** | Temperature, EEPROM config | Distance only | Distance only |
| **Power** | TTL levels | Analog out | Digital I/O |
| **Best for** | Configuration, monitoring | Simple range check | Precision timing |

---

## Complete Project Structure

```
test-stm32-urm37-rs/
├── urm37-rs/              # Library crate
│   ├── src/
│   │   ├── lib.rs
│   │   ├── uart.rs        # Blocking UART
│   │   ├── uart_async.rs  # Async UART ⭐ most common
│   │   ├── pwm.rs         # PWM mode
│   │   ├── analog.rs      # ADC conversion
│   │   └── protocol.rs    # Frame encoding
│   └── examples/
│       ├── uart_async_stm32.rs
│       ├── analog_stm32.rs
│       └── pwm_stm32.rs
└── src/main.rs            # Full working example (PWM autonomous mode)
```

---

## Running Examples

### From the library crate (urm37-rs/)

```bash
# UART mode
cargo run --example uart_async_stm32 --features async --release

# Analog mode
cargo run --example analog_stm32 --features analog --release

# PWM mode
cargo run --example pwm_stm32 --features pwm --release
```

### From the main project (test-stm32-urm37-rs/)

```bash
cargo build --release
# Flash to board
```

---

## Troubleshooting

### UART: All timeouts
→ Check baud rate (9600 by default), sensor mode (passive), and RX timeout duration.

### Analog: Readings way too high
→ Verify PA4 pin, ADC channel selection, and VCC voltage.

### PWM: Inconsistent readings
→ Ensure sensor is in **autonomous mode** (0xAA).  
→ Timer frequency must be 1 MHz for accurate µs measurement.  
→ PulseReader synchronization is critical (wait for rising edge first).

---

## EEPROM Configuration

Change sensor mode via UART:

```rust
use urm37::EepromRegister;

// Autonomous mode: sensor measures continuously
sensor.eeprom_write(EepromRegister::MeasureMode, 0xAA).await?;

// Passive mode: sensor waits for TRIG
sensor.eeprom_write(EepromRegister::MeasureMode, 0xBB).await?;

// Configure thresholds (comparator output)
sensor.set_comp_threshold(50).await?;  // Trigger at 50 cm

// Check current mode
let mode = sensor.eeprom_read(EepromRegister::MeasureMode).await?;
println!("Mode: {:#04x}", mode);
```
