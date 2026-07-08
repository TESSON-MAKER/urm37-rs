# urm37 Documentation

Complete guide to using the `urm37` embedded driver for the DFRobot URM37 V4.0 ultrasonic sensor.

---

## Quick Start

### Installation

```toml
# Default (blocking UART)
urm37 = "0.10"

# Async UART (Embassy, RTIC)
urm37 = { version = "0.10", features = ["async"] }

# Multiple modes
urm37 = { version = "0.10", features = ["blocking", "async", "pwm", "analog"] }
```

### Reading Distance

**Blocking:**
```rust
use urm37::uart::Urm37Uart;

let mut sensor = Urm37Uart::new(uart);
let distance = sensor.read_distance()?;
```

**Async:**
```rust
use urm37::uart_async::Urm37UartAsync;

let mut sensor = Urm37UartAsync::new(uart);
let distance = sensor.read_distance().await?;
```

**PWM:**
```rust
use urm37::pwm::Urm37Pwm;

let mut sensor = Urm37Pwm::new(trig_pin)?;
let distance = sensor.measure(&mut delay, || async {
    measure_echo_us().await
}).await?;
```

**Analog:**
```rust
use urm37::analog::adc_to_distance_cm;

let raw: u16 = adc.read(&mut pin)?;
let distance = adc_to_distance_cm(raw, 4095); // 12-bit
```

---

## Modules

### `protocol` — Wire Protocol

Handles URM37 UART frame encoding/decoding.

**Types:**
- `Frame` — 4-byte command/response with automatic checksum
- `Command` — Enum of commands (Temperature, Distance, EepromRead, EepromWrite)
- `EepromRegister` — Sensor configuration addresses
- `MeasureMode` — Auto (0xAA) or Passive (0xBB)
- `SerialLevelMode` — TTL or RS232

**Functions:**
- `encode_threshold(distance_cm)` → (high, low) bytes
- `decode_threshold(high, low)` → distance_cm

**Example:**
```rust
use urm37::protocol::Frame;

let frame = Frame::distance_request();
let response = [0x22, 0x01, 0x2C, 0x4F];
let parsed = Frame::parse(response)?;
let distance = parsed.decode_distance()?;
```

---

### `error` — Error Types

```rust
pub enum Error<E> {
    Bus(E),                              // HAL error
    ChecksumMismatch { expected, got },  // Corrupt frame
    UnexpectedResponse,                   // Wrong response
    InvalidReading,                       // Sensor returned 0xFFFF
    Timeout,                              // No response
}
```

**Usage:**
```rust
match sensor.read_distance() {
    Ok(cm) => println!("Distance: {} cm", cm),
    Err(Error::InvalidReading) => println!("Out of range"),
    Err(Error::Bus(e)) => println!("UART error: {:?}", e),
    Err(e) => println!("Error: {:?}", e),
}
```

---

### `uart` — Blocking UART (feature: `blocking`)

Synchronous driver for any `embedded_io::Read + Write` type.

**Methods:**
```rust
pub struct Urm37Uart<UART, E> { /* ... */ }

impl<UART, E> Urm37Uart<UART, E> {
    pub fn new(uart: UART) -> Self;
    pub fn read_distance(&mut self) -> Result<u16, Error<E>>;
    pub fn read_temperature(&mut self) -> Result<f32, Error<E>>;
    pub fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>>;
    pub fn eeprom_write(&mut self, reg: EepromRegister, val: u8) -> Result<(), Error<E>>;
}
```

**Example:**
```rust
let mut sensor = Urm37Uart::new(uart);
loop {
    match sensor.read_distance() {
        Ok(cm) => println!("Distance: {} cm", cm),
        Err(e) => println!("Error: {:?}", e),
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
}
```

---

### `uart_async` — Async UART (feature: `async`)

Asynchronous driver for any `embedded_io_async::Read + Write` type.

**Methods:** (same as `uart`, but all `async`)
```rust
pub async fn read_distance(&mut self) -> Result<u16, Error<E>>;
pub async fn read_temperature(&mut self) -> Result<f32, Error<E>>;
pub async fn eeprom_read(&mut self, reg: EepromRegister) -> Result<u8, Error<E>>;
pub async fn eeprom_write(&mut self, reg: EepromRegister, val: u8) -> Result<(), Error<E>>;
```

**Concurrent Measurements:**
```rust
use embassy_futures::join::join;

let (dist, temp) = join(
    sensor.read_distance(),
    sensor.read_temperature(),
).await;
```

---

### `pwm` — PWM Trigger Mode (feature: `pwm`)

Pulse-width based distance measurement.

**How it works:**
1. Driver pulls TRIG low for 15 µs (triggers measurement)
2. Sensor responds with ECHO pulse proportional to distance
3. Caller measures ECHO pulse width with their timer
4. Distance = pulse_width_µs / 50

**Type:**
```rust
pub struct Urm37Pwm<TRIG> { /* ... */ }

impl<TRIG> Urm37Pwm<TRIG> {
    pub fn new(trig_pin: TRIG) -> Result<Self, TRIG::Error>;
    pub async fn measure<D, F, Fut>(
        &mut self, 
        delay: &mut D, 
        echo_capture: F
    ) -> Result<Option<u16>, Error<()>>
    where
        D: DelayNs,
        F: FnOnce() -> Fut,
        Fut: Future<Output = u32>;
}
```

**Example (Embassy STM32):**
```rust
let mut sensor = Urm37Pwm::new(trig_pin)?;
let distance = sensor.measure(&mut Delay, || async {
    let (rise, fall) = join(
        ic.capture(Channel::Ch1),  // Rising edge
        ic.capture(Channel::Ch2),  // Falling edge
    ).await;
    fall.wrapping_sub(rise)
}).await?;
```

---

### `analog` — ADC Conversion (feature: `analog`)

Convert ADC readings to distance (no protocol overhead).

**Sensor outputs:** 0–800 cm mapped linearly to 0–Vcc

**Functions:**
```rust
// ADC → distance (most common)
pub fn adc_to_distance_cm(adc_raw: u16, adc_max: u16) -> u16;

// distance → ADC (reverse conversion)
pub fn distance_cm_to_adc(distance_cm: u16, adc_max: u16) -> u16;

// voltage (mV) → distance
pub fn voltage_mv_to_distance_cm(voltage_mv: u32, vcc_mv: u32) -> u16;
```

**Example:**
```rust
use urm37::analog::adc_to_distance_cm;

// 12-bit ADC (0–4095) on 3.3V system
let raw: u16 = adc.read(&mut pin)?;
let distance = adc_to_distance_cm(raw, 4095);
```

---

## Configuration

### EEPROM Registers

| Register | Address | Values | Purpose |
|----------|---------|--------|---------|
| `LargerDist` | 0x00 | 0–255 cm | COMP pulls low when distance ≥ value |
| `LessDist` | 0x01 | 0–255 cm | COMP pulls low when distance ≤ value |
| `MeasureMode` | 0x02 | 0xAA = Auto, 0xBB = Passive | Measurement mode |
| `SerialLevelMode` | 0x03 | 0x00 = TTL, 0x01 = RS232 | UART voltage level |
| `AutoMeasureTime` | 0x04 | 0–255 (×25 ms) | Interval in auto mode |

### Example: Enable Auto Mode

```rust
use urm37::protocol::{EepromRegister, MeasureMode};

// Measure every 100 ms (0x04 = 4 × 25 ms)
sensor.eeprom_write(EepromRegister::MeasureMode, MeasureMode::Auto as u8)?;
sensor.eeprom_write(EepromRegister::AutoMeasureTime, 0x04)?;
```

---

## Error Handling

### Match on Specific Errors

```rust
use urm37::Error;

match sensor.read_distance() {
    Ok(cm) => { /* success */ },
    Err(Error::InvalidReading) => { /* out of range */ },
    Err(Error::Timeout) => { /* no response */ },
    Err(Error::ChecksumMismatch { .. }) => { /* corruption */ },
    Err(Error::Bus(hal_error)) => { /* UART error */ },
    Err(Error::UnexpectedResponse) => { /* wrong response */ },
}
```

### Retry Pattern

```rust
let mut retries = 3;
let distance = loop {
    match sensor.read_distance().await {
        Ok(cm) => break cm,
        Err(e) if retries > 0 => {
            retries -= 1;
            Timer::after_millis(100).await;
        }
        Err(e) => return Err(e),
    }
};
```

---

## Platform Integration

### Embassy STM32 (Async)

Use the built-in `EmbassyUartAdapter`:

```rust
use embassy_stm32::usart::Uart;
use urm37::adapters_embassy::EmbassyUartAdapter;
use urm37::uart_async::Urm37UartAsync;

let uart = Uart::new(/* ... */);
let adapter = EmbassyUartAdapter::new(uart);
let mut sensor = Urm37UartAsync::new(adapter);

let distance = sensor.read_distance().await?;
```

### Custom Adapter (Other HALs)

Implement `embedded_io::Read + Write` for your UART:

```rust
use embedded_io::{Read, Write, ErrorType};

pub struct MyAdapter<T>(pub T);

impl<T, E> ErrorType for MyAdapter<T>
where
    T: Read<Error = E> + Write<Error = E>,
{
    type Error = E;
}

impl<T, E> Read for MyAdapter<T>
where
    T: Read<Error = E> + Write<Error = E>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() { return Ok(0); }
        self.0.read(&mut buf[..1])  // Read 1 byte at a time (protocol requirement)
    }
}

impl<T, E> Write for MyAdapter<T>
where
    T: Read<Error = E> + Write<Error = E>,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}
```

Use it:
```rust
let uart = my_hal::init_uart();
let adapter = MyAdapter(uart);
let mut sensor = Urm37Uart::new(adapter);
```

See [ADAPTERS.md](ADAPTERS.md) for more templates.

---

## Specifications

| Parameter | Value |
|-----------|-------|
| **Power Supply** | 3.3–5.0 V |
| **Operating Current** | < 20 mA |
| **Measuring Range** | 5–500 cm (PWM: 0–800 cm) |
| **Resolution** | 1 cm |
| **UART Baud Rate** | 9600 bps |
| **UART Format** | 8N1 (8 bits, no parity, 1 stop bit) |
| **Temperature Range** | −10 to +70 °C |
| **PWM Timing** | 50 µs per cm (ECHO pulse width) |
| **Analog Output** | 6.8 mV per cm (DAC pin) |

---

## Troubleshooting

### No Response from Sensor
- Check power supply (3.3–5.0 V, ~15 mA during measurement)
- Verify UART baud rate (9600 bps)
- Confirm RX/TX not reversed
- Check TTL vs RS232 mode matches your microcontroller
- Press button on sensor 1 second to toggle mode, then power cycle

### Checksum Errors
- Check for EMI/RFI near sensor
- Use shielded cable for UART
- Add 100 nF capacitor across sensor power pins
- Verify stable power supply

### Invalid Readings (0xFFFF)
- Place target at 10–30 cm (optimal range)
- Use hard, reflective surface
- Aim perpendicular to target
- Clean sensor lens
- Check ambient temperature (auto-compensated)

### EEPROM Changes Not Persisting
- Verify write succeeded (check return value)
- Power cycle sensor after write
- Read back value to confirm: `eeprom_read(reg)`

### Inaccurate Distance
- Verify sensor is in correct mode (TTL vs RS232)
- Check ADC resolution parameter if using analog mode
- Measure reference object at known distance
- Typical accuracy: ±2 cm (10–300 cm range), ±3 cm (300–500 cm)

---

## Feature Flags

| Feature | Default | Includes | Use When |
|---------|---------|----------|----------|
| `blocking` | Yes | `uart::Urm37Uart` | Simple blocking code |
| `async` | No | `uart_async::Urm37UartAsync` | Using async runtime (Embassy, RTIC) |
| `pwm` | No | `pwm::Urm37Pwm` | Measuring via ECHO pulse width |
| `analog` | No | `analog::adc_to_distance_cm()` | Reading analog DAC output |
| `defmt` | No | `defmt::Format` on errors | Using defmt logging |

---

## Pin Configuration

| Pin | Label | Mode | Description |
|-----|-------|------|-------------|
| 1 | VCC | In | Power (3.3–5.0 V) |
| 2 | GND | — | Ground |
| 3 | NRST | In | Reset (active low) |
| 4 | ECHO | Out | PWM: pulse width ∝ distance |
| 5 | MOTO | Out | Servo control (rarely used) |
| 6 | COMP/TRIG | In/Out | Comparator or PWM trigger |
| 7 | DAC | Out | Analog: 6.8 mV per cm |
| 8 | RXD | In | Serial receive |
| 9 | TXD | Out | Serial transmit |

---

## Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Distance (UART) | ~30 ms | 4 ms TX + ~25 ms sensor |
| Temperature (UART) | ~30 ms | Same as distance |
| EEPROM read | ~50 ms | |
| EEPROM write | ~50 ms | Sensor echoes to confirm |
| PWM pulse | 20–40 ms | Depends on distance |
| Pulse width | 50 µs/cm | 100 cm = 5000 µs |
| ADC conversion | 1–100 µs | Depends on ADC resolution |

---

## Resources

- [README.md](README.md) — Quick start and hardware specs
- [ADAPTERS.md](ADAPTERS.md) — HAL adapter templates
- [DFRobot Datasheet](https://wiki.dfrobot.com/URM37_V4.0_Ultrasonic_Sensor) — Hardware specifications
- [GitHub Issues](https://github.com/TESSON-MAKER/urm37-rs) — Report bugs or ask questions
- [Rust Embedded Book](https://rust-embedded.org/) — General embedded Rust knowledge

---

**License:** MIT OR Apache-2.0  
**Crate Version:** 0.10.0
