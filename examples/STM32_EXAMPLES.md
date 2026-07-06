# URM37 STM32 Examples with Embassy

Complete, production-ready examples for using the URM37 ultrasonic sensor on STM32 microcontrollers with the Embassy async runtime.

## Overview

| Example | Mode | Features |
|---------|------|----------|
| **stm32_uart_async** | Async UART | Simple distance + temperature reading |
| **stm32_pwm_mode** | PWM trigger | Input capture, high precision timing |
| **stm32_analog_mode** | Analog (ADC) | Voltage-to-distance conversion |
| **stm32_eeprom_config** | Async UART | Configuration, threshold setup, auto-mode |
| **stm32_complete** | Async UART | Error handling, retries, statistics, monitoring |

## Prerequisites

### Toolchain
```bash
rustup target add thumbv7em-none-eabihf  # For STM32L4, STM32H7, etc.
cargo install cargo-flash
```

### Dependencies (in Cargo.toml)
```toml
[dev-dependencies]
embassy-executor = { version = "0.5", features = ["arch-cortex-m"] }
embassy-stm32 = { version = "0.1", features = ["stm32l476rg"] }  # Adjust for your MCU
embassy-time = "0.3"
embassy-futures = "0.1"
urm37 = { version = "0.4", features = ["uart-async"] }
defmt = "0.3"
defmt-rtt = "0.4"
```

## Quick Start

### 1. Simple Async UART (stm32_uart_async)

**Best for:** Getting started quickly, simple distance/temperature reading.

```bash
cargo run --example stm32_uart_async --target thumbv7em-none-eabihf --features uart-async
```

**Hardware:**
- USART3: PA10 (RX) ↔ URM37 TXD
- USART3: PA9 (TX) ↔ URM37 RXD
- 3.3V power supply

**Output:**
```
Distance: 125 cm
Temperature: 23.5 °C
Distance: 127 cm
Temperature: 23.6 °C
```

### 2. PWM Mode with Input Capture (stm32_pwm_mode)

**Best for:** Precise distance measurement, 50 µs/cm resolution.

```bash
cargo run --example stm32_pwm_mode --target thumbv7em-none-eabihf --features pwm
```

**Hardware:**
- PA0 (GPIO output) → URM37 COMP/TRIG
- PA5 (TIM2_CH1) ← URM37 ECHO (rising edge)
- PA1 (TIM2_CH2) ← URM37 ECHO (falling edge)

**Features:**
- Concurrent rising/falling edge capture using `embassy_futures::join::join`
- 1 µs timer resolution (1 MHz clock)
- Automatic pulse width → distance conversion

### 3. Analog Mode (stm32_analog_mode)

**Best for:** Simple setup, no UART required, lowest latency.

```bash
cargo run --example stm32_analog_mode --target thumbv7em-none-eabihf --features analog
```

**Hardware:**
- PA6 (ADC1_IN6) ← URM37 DAC_OUT

**Resolution:**
- 12-bit ADC: ~2 cm per 10 LSBs
- Linear: 0V = 0 cm, 3.3V = 800 cm

### 4. EEPROM Configuration (stm32_eeprom_config)

**Best for:** Setting up the sensor parameters on startup.

```bash
cargo run --example stm32_eeprom_config --target thumbv7em-none-eabihf --features uart-async
```

**Configuration examples:**
```rust
// Set COMP threshold to 50 cm
sensor.set_comp_threshold(50).await?;

// Auto-measure every 500ms (20 × 25ms intervals)
sensor.set_auto_mode(20).await?;

// Return to passive (on-demand) mode
sensor.set_passive_mode().await?;
```

### 5. Production-Ready Example (stm32_complete)

**Best for:** Real applications with error handling, monitoring, statistics.

```bash
cargo run --example stm32_complete --target thumbv7em-none-eabihf --features uart-async
```

**Features:**
- Automatic retry logic (3 attempts per measurement)
- Success rate statistics
- Min/max distance tracking
- COMP pin threshold detection
- Graceful error handling
- 10-second periodic statistics

**Output:**
```
╔════════════════════════════════════════╗
║  URM37 Complete Example - STM32 Board  ║
╚════════════════════════════════════════╝
✓ COMP threshold: 50 cm
✓ Auto-measurement: enabled (1 second interval)
✓ Sensor ready
Distance: 125 cm  |  Temp: 23.5°C
Distance: 126 cm  |  Temp: 23.5°C
Stats: 10/10 OK, Range: 120-130 cm, Success: 100%
```

## Hardware Wiring

### UART Mode (Recommended for beginners)

```
STM32          URM37
═════          ═════
PB10 (TX) ──→ Pin 8 (RXD)
PB11 (RX) ←── Pin 9 (TXD)
3.3V      ──→ Pin 1 (VCC)
GND       ──→ Pin 2 (GND)
```

### PWM Mode (High precision)

```
STM32                    URM37
═════                    ═════
PA0 (GPIO out)       ──→ Pin 6 (COMP/TRIG)
PA5 (TIM2_CH1 rising) ←── Pin 4 (ECHO)
PA1 (TIM2_CH2 fall)   ←── Pin 4 (ECHO)
3.3V                  ──→ Pin 1 (VCC)
GND                   ──→ Pin 2 (GND)
```

### Analog Mode (Simplest)

```
STM32          URM37
═════          ═════
PA6 (ADC1) ←── Pin 7 (DAC_OUT)
3.3V       ──→ Pin 1 (VCC)
GND        ──→ Pin 2 (GND)
```

## Common Issues & Solutions

### Issue: UART times out or checksum errors
- **Cause:** Baud rate mismatch (URM37 defaults to 9600)
- **Solution:** Verify `config.baudrate = 9600` in example

### Issue: PWM measurement returns None
- **Cause:** ECHO pulse not being captured
- **Solution:** 
  1. Check pin connections (PA5, PA1)
  2. Verify timer is running at 1 MHz
  3. Test with known distance (e.g., 50 cm tape)

### Issue: ADC reads all zeros or max values
- **Cause:** Incorrect ADC channel or pin not connected
- **Solution:**
  1. Verify PA6 → DAC_OUT connection
  2. Check ADC is configured for channel 6
  3. Test with multimeter on DAC_OUT pin

### Issue: defmt output not showing
- **Cause:** defmt-rtt not initialized
- **Solution:** 
  1. Ensure `defmt-rtt` in dev-dependencies
  2. Use probe-rs/J-Link for RTT viewer
  3. Check `cortex-m-rt` features enabled

## Performance Characteristics

### UART Mode
- **Latency:** ~10-50 ms (depends on UART buffer)
- **Baud rate:** 9600 (standard)
- **Resolution:** 1 cm (sensor hardware limit)

### PWM Mode
- **Latency:** ~50-100 ms
- **Resolution:** 1 cm (50 µs pulse)
- **Precision:** 1 µs timing accuracy possible with Embassy

### Analog Mode
- **Latency:** <1 ms
- **Resolution:** ~2 cm @ 12-bit ADC
- **Advantage:** No UART latency, lowest power

## Advanced Configuration

### Custom USART Pins
```rust
let uart = Uart::new_blocking(
    p.USART3,
    p.PB11,  // RX pin (adjust for your board)
    p.PB10,  // TX pin
    p.DMA1_CH3,
    p.DMA1_CH2,
    config,
);
```

### Different STM32 Families
- **STM32L4:** `embassy-stm32 = { features = ["stm32l476rg"] }`
- **STM32H7:** `embassy-stm32 = { features = ["stm32h743vi"] }`
- **STM32F4:** `embassy-stm32 = { features = ["stm32f407vg"] }`

### Adding Your Board
Check [Embassy documentation](https://docs.embassy.dev/embassy-stm32/latest/) for your MCU features.

## References

- [URM37 V5.0 Datasheet](https://wiki.dfrobot.com/URM37%20V5.0%20Ultrasonic%20Distance%20Sensor)
- [Embassy Documentation](https://docs.embassy.dev/)
- [STM32 HAL Features](https://docs.embassy.dev/embassy-stm32/latest/embassy_stm32/)
- [defmt Logging](https://docs.rust-embedded.org/defmt/)

## License

These examples are provided under the same license as the urm37 crate (MIT OR Apache-2.0).
