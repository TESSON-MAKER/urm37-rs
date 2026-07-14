# urm37
<p align="left">
  <a href="https://crates.io/crates/urm37"><img src="https://img.shields.io/crates/v/urm37.svg" alt="crates.io" height="20"></a>
  <a href="https://docs.rs/urm37"><img src="https://docs.rs/urm37/badge.svg" alt="docs.rs" height="20"></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt="License" height="20"></a>
</p>

`no_std` embedded driver for the **DFRobot URM37 V4.0** ultrasonic distance sensor.

<div align="center">
  <img src="urm37v4-0.png" alt="DFRobot URM37 V4.0 Ultrasonic Sensor" width="350">
</div>

An industrial-grade ultrasonic sensor offering advanced capabilities with improved accuracy, temperature correction, and versatile output modes. Supports all interface modes: synchronous/asynchronous UART, PWM trigger, and analog (DAC).

---

## Key Features (V4.0)

- **Serial Level Selection** — Onboard button to switch between RS232 and TTL modes (takes effect after reboot)
- **Improved Algorithm** — Reduced dead zone and enhanced accuracy
- **Analog Voltage Output** — DAC output directly proportional to measured distance (6.8 mV/cm)
- **Wide Voltage Support** — Operating range 3.3 V to 5.0 V
- **Hardware Safety** — Integrated power reverse protection
- **Configurable Timing** — Automatic measurement interval customizable via EEPROM
- **Servo Control** — 0–180° angle mapping (compatible with standard servos)
- `no_std` — works on any microcontroller
- Synchronous UART via [`embedded-io`](https://crates.io/crates/embedded-io)
- Asynchronous UART via [`embedded-io-async`](https://crates.io/crates/embedded-io-async) (Embassy, RTIC…)
- PWM conversion: ECHO pulse → distance in cm
- Analog conversion: raw DAC voltage → distance in cm
- Temperature reading with 0.1 °C resolution (UART mode)
- Internal EEPROM configuration (thresholds, mode, timing interval)
- Optional `defmt` support for embedded logging
- Zero dynamic allocation (heapless-free)

---

## Specifications

| Parameter                  | Value                        |
|----------------------------|------------------------------|
| **Power Supply**           | 3.3 V – 5.0 V               |
| **Operating Current**      | < 20 mA                     |
| **Operating Temperature**  | −10 °C to +70 °C            |
| **Detecting Range**        | 5 cm – 500 cm               |
| **Resolution**             | 1 cm                        |
| **Communication**          | RS232 / TTL (selectable), PWM, DAC |
| **Dimensions**             | 22 mm × 51 mm              |
| **Weight**                 | 25 g                        |

### Accuracy & Timing
- **PWM Mode (ECHO):** 50 µs per 1 cm (0–25000 µs pulse width)
- **Analog Mode (DAC):** 6.8 mV per 1 cm
- **Default Auto Interval:** 25 ms
- **Temperature Coefficient:** Automatic correction via on-chip sensor

---

## Integration with STM32 & Embassy

See **[EXAMPLES.md](EXAMPLES.md)** for:

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

## Pin Configuration

| Pin | Label   | Description                                                              |
|-----|---------|--------------------------------------------------------------------------|
| 1   | VCC     | Power input (reference +5 V, accepts 3.3 V – 5.0 V)                    |
| 2   | GND     | Ground                                                                   |
| 3   | NRST    | Reset (active low)                                                       |
| 4   | ECHO    | PWM output: pulse width ∝ distance (50 µs = 1 cm, range 0–25000 µs)    |
| 5   | MOTO    | Servo motor control output (0–180° angle mapping)                       |
| 6   | COMP/TRIG | **COMP:** Pulls low when distance < threshold (comparator mode)         |
|     |         | **TRIG:** PWM trigger input for single measurements                     |
| 7   | DAC     | Analog voltage output (6.8 mV per 1 cm)                               |
| 8   | RXD     | Serial data receive (RS232 / TTL level, configurable)                  |
| 9   | TXD     | Serial data transmit (RS232 / TTL level, configurable)                 |

> **WARNING:** Select RS232 or TTL mode via the on-board button before wiring.  
> **Never** connect a TTL MCU while the sensor is in RS232 mode — permanent damage will result.  
> Default: **TTL level** (LED flashes: 1 long + 1 short). Press button 1 second (until LED off), then cycle power.

---

## Communication Protocol

### Serial Settings
- **Baud Rate:** 9600 bps
- **Parity:** None
- **Stop Bits:** 1
- **Data Bits:** 8

### Frame Format
All commands consist of **4 bytes:** `[Command] [Data0] [Data1] [SUM]`

`SUM` = low 8 bits of the sum of the first 3 bytes (checksum).

### Command Reference

#### Live Measurement Commands

| Operation | Frame | Response | Notes |
|-----------|-------|----------|-------|
| **Read Distance** | `0x22 Deg 0x00 SUM` | `0x22 High Low SUM` | Distance (cm) = `(High × 256) + Low`. Returns `0xFF 0xFF` if invalid. `Deg` drives servo (0x00 if unused). |
| **Read Temperature** | `0x11 0x00 0x00 0x11` | `0x11 High Low SUM` | 0.1 °C resolution. High byte bits [7:4]: if 0 → positive, if 1 (0xF0) → negative. Returns `0xFF 0xFF` if invalid. |

#### EEPROM Access Commands

| Operation | Frame | Response | Notes |
|-----------|-------|----------|-------|
| **Read EEPROM** | `0x33 Add 0x00 SUM` | `0x33 Add Data SUM` | Reads configuration value at address `Add`. |
| **Write EEPROM** | `0x44 Add Data SUM` | `0x44 Add Data SUM` | Sensor echoes the frame to confirm successful write. |

### EEPROM Memory Map (Configuration Registers)

| Address | Name | Values | Purpose |
|---------|------|--------|---------|
| `0x00` | **Low Threshold** | 0x00–0xFF (cm) | COMP pin triggers low if distance **≥** this value |
| `0x01` | **High Threshold** | 0x00–0xFF (cm) | COMP pin triggers low if distance **≤** this value |
| `0x02` | **Operating Mode** | `0xAA` = Autonomous, other = Passive PWM | Controls measurement behavior |
| `0x03` | **Serial Level** | `0x00` = TTL, `0x01` = RS232 | Selects UART signal voltage |
| `0x04` | **Time Interval** | 25–255 ms (hex value) | Polling delay in Autonomous mode; `0x64` = 100 ms |

**Default factory values:** All registers initialized to `0x00`.

### Measurement Modes

1. **PWM Triggered Mode**  
   Host sends a low pulse (> 1 µs) on COMP/TRIG pin. Sensor responds with ECHO pulse width encoding distance.

2. **Autonomous (Automatic) Mode**  
   Sensor automatically measures at user-defined intervals (register `0x04`). If measured distance ≤ High Threshold **or** ≥ Low Threshold, COMP pin pulls low (ultrasonic switch behavior).

3. **Serial Passive Mode**  
   Host MCU queries sensor via UART commands (0x22 for distance, 0x11 for temperature).

### Servo Rotation Mapping

The MOTO pin accepts angle codes (0x00–0x1E) that map to 0–176°:

| Hex | Deg | Hex | Deg | Hex | Deg | Hex | Deg |
|-----|-----|-----|-----|-----|-----|-----|-----|
| 0x00 | 0° | 0x01 | 6° | 0x02 | 12° | 0x03 | 18° |
| 0x04 | 24° | 0x05 | 29° | 0x06 | 35° | 0x07 | 41° |
| 0x08 | 47° | 0x09 | 53° | 0x0A | 59° | 0x0B | 65° |
| 0x0C | 70° | 0x0D | 76° | 0x0E | 82° | 0x10 | 94° |
| 0x11 | 100° | 0x12 | 106° | 0x13 | 112° | 0x14 | 117° |
| 0x15 | 123° | 0x16 | 129° | 0x17 | 135° | 0x18 | 141° |
| 0x19 | 147° | 0x1A | 153° | 0x1B | 159° | 0x1C | 164° |
| 0x1D | 170° | 0x1E | 176° | — | — | — | — |

---

## Standardized Output Format

All examples follow this consistent output format for easy parsing and monitoring:

```
[DISTANCE] X cm              # Successful distance measurement
[TEMPERATURE] X.X °C         # Temperature reading
[OUT_OF_RANGE]               # Sensor reading out of valid range
[ERROR]                       # Communication or sensor error
```

This format enables:
- Easy serial port monitoring
- Simple regex-based parsing
- Scripted data collection
- Cross-platform compatibility

---

## Examples

Ready-to-use examples for popular microcontrollers and frameworks:

### Arduino Mega 2560

#### 1. UART Mode (`examples/mega2560_uart.rs`)

Dual UART: one for computer, one for sensor.

**Hardware:**
- Arduino Mega 2560
- USART0 (D0/D1): Computer communication (57600 baud)
- USART1 (D18/D19): URM37 sensor (9600 baud)

**Run:**
```bash
cargo build --example mega2560_uart --features blocking
```

#### 2. PWM Mode (`examples/mega2560_pwm.rs`)

High-precision distance measurement using PWM pulse.

**Hardware:**
- Arduino Mega 2560
- D9: TRIG output
- D2: ECHO input (pulse measurement)

**Run:**
```bash
cargo build --example mega2560_pwm --features pwm
```

#### 3. Analog Mode (`examples/mega2560_analog.rs`)

Simple analog voltage-to-distance conversion.

**Hardware:**
- Arduino Mega 2560
- A0: Analog voltage input (6.8 mV/cm)

**Run:**
```bash
cargo build --example mega2560_analog --features analog
```

---

### STM32F767ZI (Nucleo) with Embassy

#### 1. Async UART Mode (`examples/stm32_uart_async.rs`)

Asynchronous UART communication with distance and temperature.

**Hardware:**
- STM32F767ZI (Nucleo F767ZI)
- UART5: RX=PD2, TX=PC12 (DMA: CH0 TX, CH7 RX)

**Run:**
```bash
cargo run --example stm32_uart_async --features async --release
```

#### 2. Async PWM Mode (`examples/stm32_pwm.rs`)

High-precision async PWM with InputCapture.

**Hardware:**
- STM32F767ZI (Nucleo F767ZI)
- PA0: TRIG output (GPIO)
- PA5: ECHO input (TIM2 CH1 InputCapture)

**Run:**
```bash
cargo run --example stm32_pwm --features pwm --release
```

#### 3. Async ADC Mode (`examples/stm32_analog.rs`)

Simple async ADC reading for distance.

**Hardware:**
- STM32F767ZI (Nucleo F767ZI)
- PA4: Analog voltage input

**Run:**
```bash
cargo run --example stm32_analog --features analog --release
```

**Features:**
- 12-bit ADC reading
- Direct ADC-to-distance conversion
- No UART or timing logic required (simplest option)

---

## Installation

```toml
[dependencies]
# Choose the features you need:
urm37 = { version = "1.1", features = ["uart-async"] }
# or
urm37 = { version = "1.1", features = ["uart", "pwm", "analog"] }
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

The PWM driver automatically manages the TRIG pin and provides two modes based on sensor configuration:

#### Asynchronous PWM (Embassy-based, recommended for async code)

```rust
use urm37::pwm_async::{Urm37PwmAsync, PulseReaderAsync};
use embedded_hal_async::delay::DelayNs;

// Implement PulseReaderAsync for your timer/input-capture hardware
struct MyPulseReader { /* your IC setup */ }

impl PulseReaderAsync for MyPulseReader {
    async fn measure_pulse(&mut self) -> Option<u32> {
        // Return pulse width in µs (0-50000)
        // Measure ECHO LOW pulse with microsecond precision
    }
}

let mut sensor = Urm37PwmAsync::new(trig_pin, pulse_reader, delay)?;
sensor.set_trigger_duration(10); // 10 ms pulse

// Autonomous mode (sensor auto-measures)
match sensor.read_distance().await {
    Ok(Some(cm)) => println!("Distance: {} cm", cm),
    Ok(None) => println!("Out of range"),
    Err(e) => println!("Error: {:?}", e),
}

// Passive mode (manual TRIG)
match sensor.read_distance_manual().await {
    Ok(Some(cm)) => println!("Distance: {} cm", cm),
    Ok(None) => println!("Out of range"),
    Err(e) => println!("Error: {:?}", e),
}
```

#### Synchronous PWM (blocking, no async/await)

```rust
use urm37::pwm::{Urm37Pwm, PulseReader};
use embedded_hal::delay::DelayNs;

// Implement PulseReader (blocking version)
struct MyPulseReader { /* GPIO + timer */ }

impl PulseReader for MyPulseReader {
    fn measure_pulse(&mut self) -> Option<u32> {
        // Return pulse width in µs (0-50000)
        // Busy-wait for ECHO LOW pulse (blocking)
    }
}

let mut sensor = Urm37Pwm::new(trig_pin, pulse_reader, delay)?;
sensor.set_trigger_duration(10);

// Autonomous mode
match sensor.read_distance() {
    Ok(Some(cm)) => println!("Distance: {} cm", cm),
    Ok(None) => println!("Out of range"),
    Err(e) => println!("Error: {:?}", e),
}

// Passive mode
match sensor.read_distance_manual() {
    Ok(Some(cm)) => println!("Distance: {} cm", cm),
    Ok(None) => println!("Out of range"),
    Err(e) => println!("Error: {:?}", e),
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

## Choosing the Right Mode

| Mode | Pros | Cons | Best For |
|------|------|------|----------|
| **UART (async/sync)** | Full sensor control, temperature, EEPROM config | Requires serial setup, 9600 bps | Configurable systems, monitoring, telemetry |
| **PWM Async** | Non-blocking, integrates with Embassy, high precision | Requires async runtime, input capture or timer | Modern embedded async code, real-time systems |
| **PWM Sync** | Simple blocking API, no async overhead | Busy-waits on pulse, blocks task | Simple applications, straightforward pulse measurement |
| **Analog/ADC** | Simplest, no UART or special timing | Fixed 6.8 mV/cm mapping, lower precision | Cost-sensitive, simple systems, no timing requirements |

### PWM Mode Details

**Autonomous Mode (0xAA):**
- Sensor auto-measures distance at configurable intervals
- Call `read_distance()` or `read_distance()` to get latest measurement
- Simpler API, sensor handles triggering

**Passive Mode (0xBB):**
- Sensor waits for explicit TRIG pulse from MCU
- Call `read_distance_manual()` to trigger measurement and read result
- Gives you precise control over measurement timing
- Recommended for synchronization with other operations

---

## Cargo features

| Feature       | Default | Description                                           |
|---------------|---------|-------------------------------------------------------|
| `blocking`    | no      | Synchronous (blocking) UART driver                    |
| `async`       | no      | Async/await UART driver (Embassy, RTIC)              |
| `pwm`         | no      | PWM mode (both async `Urm37PwmAsync` and sync `Urm37Pwm`) |
| `analog`      | no      | Analog/ADC mode utilities                             |
| `defmt`       | no      | `defmt` logging support                               |

---

## `embedded-hal` compatibility

| Crate               | Version |
|---------------------|---------|
| `embedded-hal`      | 1.0     |
| `embedded-io`       | 0.6     |
| `embedded-io-async` | 0.6     |

---

## Troubleshooting

### Communication Failures
- **Check serial level mode:** Verify the sensor's physical serial level mode (TTL vs. RS232) matches your microcontroller interface.
- **Button configuration:** Press the on-board button for 1 second (LED turns off), then cycle power to activate mode changes.
- **Baud rate:** Ensure communication at 9600 bps, 8 data bits, no parity, 1 stop bit.

### Measurement Issues

**Unstable or invalid readings (0xFFFF returned)**
- Ultrasonic signals attenuate as `1/d²` in open environments.
- Ensure good surface alignment and target orientation.
- Soft surfaces or narrow objects (e.g., pens) may not reflect ultrasound effectively.

**ECHO pulse out of range**
- Check power supply voltage (3.3 V – 5.0 V).
- Verify ECHO pin is not floating or damaged.
- Ensure pull-up resistor on ECHO if needed by your MCU.

**COMP threshold not triggering**
- Read EEPROM registers `0x00` (low) and `0x01` (high) to confirm threshold values.
- Verify the sensor is in Autonomous mode (`0x02` = `0xAA`).
- Check the logic: COMP pulls low when distance **≤ high threshold OR ≥ low threshold**.

### Servo Control
- Angle mapping uses hex codes `0x00` (0°) to `0x1E` (176°).
- MOTO output is 5 V logic; ensure servo is compatible.
- Non-standard servo models may require PWM conditioning.

---

## License

Dual-licensed under MIT and Apache 2.0 — your choice.
