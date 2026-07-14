//! # URM37 Async UART Example for STM32F767ZI
//!
//! Demonstrates async UART communication with distance and temperature reading.
//!
//! ## Hardware Setup
//! - **UART5**: RX=PD2, TX=PC12
//! - **STM32F767ZI (Nucleo)**
//! - DMA: CH0 (TX), CH7 (RX)
//!
//! ## Output Format
//! ```text
//! [DISTANCE] X cm
//! [TEMPERATURE] X.X °C
//! [ERROR]
//! ```
//!
//! ## Build & Run
//! ```bash
//! cargo run --example stm32_uart_async --features async --release
//! ```

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::usart::Uart;
use embassy_stm32::{bind_interrupts, dma, peripherals, usart};
use embassy_time::Timer;
use urm37::uart_async::Urm37UartAsync;
use embedded_io_async::{Read, Write};
use core::fmt;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    UART5 => usart::InterruptHandler<peripherals::UART5>;
    DMA1_STREAM0 => dma::InterruptHandler<peripherals::DMA1_CH0>;
    DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
});

#[derive(Debug, Clone, Copy)]
pub enum TransportError {
    Uart(embassy_stm32::usart::Error),
    Timeout,
}

impl From<embassy_stm32::usart::Error> for TransportError {
    fn from(e: embassy_stm32::usart::Error) -> Self {
        TransportError::Uart(e)
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Uart(_) => f.write_str("UART error"),
            TransportError::Timeout => f.write_str("Timeout"),
        }
    }
}

impl core::error::Error for TransportError {}

impl embedded_io::Error for TransportError {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            TransportError::Timeout => embedded_io::ErrorKind::TimedOut,
            TransportError::Uart(_) => embedded_io::ErrorKind::Other,
        }
    }
}

impl defmt::Format for TransportError {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            TransportError::Uart(_) => defmt::write!(fmt, "Uart"),
            TransportError::Timeout => defmt::write!(fmt, "Timeout"),
        }
    }
}

struct UartWrapper<'d> {
    uart: Uart<'d, embassy_stm32::mode::Async>,
}

impl<'d> embedded_io_async::ErrorType for UartWrapper<'d> {
    type Error = TransportError;
}

impl<'d> Read for UartWrapper<'d> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        match embassy_time::with_timeout(embassy_time::Duration::from_millis(100), self.uart.read(&mut buf[..1])).await {
            Ok(Ok(())) => Ok(1),
            Ok(Err(e)) => Err(TransportError::Uart(e)),
            Err(_) => Err(TransportError::Timeout),
        }
    }
}

impl<'d> Write for UartWrapper<'d> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.uart.write_all(buf).await?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    // Configure UART5 (PC12=TX, PD2=RX) with DMA
    let mut uart_config = embassy_stm32::usart::Config::default();
    uart_config.baudrate = 9600;

    let uart = Uart::new(
        p.UART5,
        p.PD2,  // RX
        p.PC12, // TX
        p.DMA1_CH7,  // RX DMA
        p.DMA1_CH0,  // TX DMA
        Irqs,
        uart_config,
    )
    .unwrap();

    let wrapper = UartWrapper { uart };
    let mut sensor = Urm37UartAsync::new(wrapper);

    loop {
        match sensor.read_distance().await {
            Ok(distance) => {
                info!("[DISTANCE] {} cm", distance);
            }
            Err(_) => {
                info!("[ERROR]");
            }
        }

        Timer::after_millis(500).await;

        match sensor.read_temperature().await {
            Ok(temp) => {
                let temp_int = temp as i32;
                let temp_frac = ((temp * 10.0) as i32) % 10;
                info!("[TEMPERATURE] {}.{} °C", temp_int, temp_frac);
            }
            Err(_) => {
                info!("[ERROR]");
            }
        }

        Timer::after_millis(500).await;
    }
}
