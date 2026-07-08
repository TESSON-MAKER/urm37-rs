# Platform Adapters

The `urm37` crate is **HAL-agnostic** and works with any implementation of the standard `embedded-io` and `embedded-io-async` traits.

Most HALs don't directly implement these traits. Instead, you create a thin **adapter** in your application that wraps the HAL's UART and implements the required traits.

---

## Architecture

The crate requires:

- **Synchronous UART** (`feature = "uart"`): Your adapter must implement `embedded_io::Read + Write`
- **Asynchronous UART** (`feature = "uart-async"`): Your adapter must implement `embedded_io_async::Read + Write`
- **PWM mode** (`feature = "pwm"`): Uses `embedded_hal::digital::OutputPin` (standard across HALs)

---

## Embassy STM32 Adapter (Async)

**File:** `src/uart_adapter.rs`

```rust
use embassy_stm32::usart::Uart;
use embassy_stm32::mode::Async;
use embedded_io_async::{Read, Write, ErrorType};

pub struct EmbassyStm32UartAdapter<'d> {
    uart: Uart<'d, Async>,
}

impl<'d> EmbassyStm32UartAdapter<'d> {
    pub fn new(uart: Uart<'d, Async>) -> Self {
        Self { uart }
    }

    pub fn release(self) -> Uart<'d, Async> {
        self.uart
    }
}

impl<'d> ErrorType for EmbassyStm32UartAdapter<'d> {
    type Error = embassy_stm32::usart::Error;
}

impl<'d> Read for EmbassyStm32UartAdapter<'d> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Read one byte at a time (protocol requirement)
        if buf.is_empty() {
            return Ok(0);
        }

        self.uart.read(&mut buf[..1]).await?;
        Ok(1)
    }
}

impl<'d> Write for EmbassyStm32UartAdapter<'d> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.uart.write_all(buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
```

**Usage in `main.rs`:**

```rust
#![no_std]
#![no_main]

mod uart_adapter;

use uart_adapter::EmbassyStm32UartAdapter;
use urm37::uart_async::Urm37UartAsync;
use embassy_stm32::usart::Config;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    let uart = embassy_stm32::usart::Uart::new(
        p.UART5,
        p.PB8,
        p.PB9,
        p.DMA1_CH7,
        p.DMA1_CH0,
        Irqs,
        Config::default(),
    ).unwrap();

    let adapter = EmbassyStm32UartAdapter::new(uart);
    let mut sensor = Urm37UartAsync::new(adapter);

    loop {
        match sensor.read_distance().await {
            Ok(dist_cm) => println!("Distance: {} cm", dist_cm),
            Err(_) => println!("Error"),
        }
    }
}
```

---

## Arduino / AVR-HAL Adapter (Blocking)

**File:** `src/uart_adapter.rs`

```rust
use embedded_io::{Read, Write, ErrorType};

pub struct ArduinoUartAdapter<SERIAL> {
    serial: SERIAL,
}

impl<SERIAL> ArduinoUartAdapter<SERIAL> {
    pub fn new(serial: SERIAL) -> Self {
        Self { serial }
    }

    pub fn release(self) -> SERIAL {
        self.serial
    }
}

impl<SERIAL, E> ErrorType for ArduinoUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    type Error = E;
}

impl<SERIAL, E> Read for ArduinoUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Protocol requires single-byte reads
        if buf.is_empty() {
            return Ok(0);
        }

        self.serial.read(&mut buf[..1])
    }
}

impl<SERIAL, E> Write for ArduinoUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.serial.write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.serial.flush()
    }
}
```

**Usage in `main.rs` (Arduino Uno):**

```rust
use arduino_hal::prelude::*;
use urm37::uart::Urm37Uart;
use uart_adapter::ArduinoUartAdapter;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Serial port: RX=pin0 (D0), TX=pin1 (D1)
    let mut serial = arduino_hal::default_serial!(dp, pins, 9600);

    let adapter = ArduinoUartAdapter::new(serial);
    let mut sensor = Urm37Uart::new(adapter);

    loop {
        match sensor.read_distance() {
            Ok(dist_cm) => {
                ufmt::uwriteln!(&mut serial, "Distance: {} cm\r", dist_cm).ok();
            }
            Err(_) => {
                ufmt::uwriteln!(&mut serial, "Error\r").ok();
            }
        }

        arduino_hal::delay_ms(500);
    }
}
```

---

## ESP32 Adapter (Blocking with esp-idf-hal)

**File:** `src/uart_adapter.rs`

```rust
use embedded_io::{Read, Write, ErrorType};

pub struct EspUartAdapter<SERIAL> {
    serial: SERIAL,
}

impl<SERIAL> EspUartAdapter<SERIAL> {
    pub fn new(serial: SERIAL) -> Self {
        Self { serial }
    }
}

impl<SERIAL, E> ErrorType for EspUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    type Error = E;
}

impl<SERIAL, E> Read for EspUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        self.serial.read(&mut buf[..1])
    }
}

impl<SERIAL, E> Write for EspUartAdapter<SERIAL>
where
    SERIAL: Read<Error = E> + Write<Error = E>,
    E: embedded_io::Error,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.serial.write(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.serial.flush()
    }
}
```

**Usage in `main.rs` (ESP32):**

```rust
use esp_idf_sys as _;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::uart::*;
use urm37::uart::Urm37Uart;
use uart_adapter::EspUartAdapter;

fn main() {
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take().unwrap();

    let config = UartConfig::default()
        .baudrate(Hertz(9600));

    let uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio1,   // TX
        peripherals.pins.gpio3,   // RX
        None::<gpio::Gpio0>,
        None::<gpio::Gpio1>,
        &config,
    ).unwrap();

    let adapter = EspUartAdapter::new(uart);
    let mut sensor = Urm37Uart::new(adapter);

    loop {
        match sensor.read_distance() {
            Ok(dist_cm) => println!("Distance: {} cm", dist_cm),
            Err(_) => println!("Error"),
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
```

---

## PWM Mode (Embassy STM32)

For PWM mode, the driver manages the TRIG pin but delegates ECHO measurement to your code.
Use your timer's input-capture peripheral and `embassy_futures::join::join` for concurrent measurement.

**Example with embassy_stm32 InputCapture:**

```rust
use urm37::pwm::Urm37Pwm;
use embassy_futures::join::join;
use embassy_stm32::timer::input_capture::{InputCapture, CapturePin, InputCapturePolarity};
use embassy_stm32::timer::Channel;
use embassy_stm32::time::hz;
use embassy_time::Delay;

// TRIG → PA0, ECHO → PA5 (rising) + PA1 (falling) on TIM2
let trig_pin = Output::new(p.PA0, Level::High, Speed::Low);
let mut sensor = Urm37Pwm::new(trig_pin).unwrap();

let mut ic = InputCapture::new(
    p.TIM2,
    Some(CapturePin::new_ch1(p.PA5)),  // Rising edge
    Some(CapturePin::new_ch2(p.PA1)),  // Falling edge
    None,
    None,
    hz(1_000_000),  // 1 tick = 1 µs
    embassy_stm32::timer::low_level::CountingMode::EdgeAlignedUp,
);

ic.set_input_capture_polarity(Channel::Ch1, InputCapturePolarity::Rising);
ic.set_input_capture_polarity(Channel::Ch2, InputCapturePolarity::Falling);

loop {
    let distance = sensor.measure(&mut Delay, || async {
        // TRIG and ECHO measurement run concurrently
        let (t_rise, t_fall) = join(
            ic.capture(Channel::Ch1),
            ic.capture(Channel::Ch2),
        ).await;
        t_fall.wrapping_sub(t_rise)
    }).await.unwrap();

    match distance {
        Some(cm) => println!("Distance: {} cm", cm),
        None => println!("Out of range"),
    }

    Timer::after_millis(100).await;
}
```

---

## Key Implementation Notes

1. **Single-byte reads**: The URM37 protocol requires reading one byte at a time. Many async HAL `read` methods return multiple bytes—wrap them to read `&mut buf[..1]`.

2. **Concurrency in PWM mode**: The `measure` method runs the TRIG pulse and ECHO capture in parallel using `embassy_futures::join::join`. This ensures input capture is armed at the exact moment the TRIG pulse begins, minimizing latency.

3. **Platform-agnostic design**: By implementing standard traits, adapters remain small and maintainable. The crate focuses on the protocol; the adapter focuses on the HAL.

4. **Error types**: Each adapter's error type is determined by the underlying HAL UART. The crate is generic over errors, so any `embedded_io::Error` implementation works.
