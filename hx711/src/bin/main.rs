#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, Delay};
use esp_backtrace as _;
use esp_hal::{
    spi::master::{Config as SpiConfig, Spi},
    spi::Mode,
    time::Rate,
    timer::timg::TimerGroup,
};
use hx711_spi::Hx711;
use embedded_hal_async::delay::DelayNs;

esp_bootloader_esp_idf::esp_app_desc!();

const CALIBRATION_SCALE: f32 = 4044.0;

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // SPI para HX711
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_khz(1000))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO12)
    .with_mosi(peripherals.GPIO14)
    .with_miso(peripherals.GPIO13);

    let mut hx711 = Hx711::new(spi);
    let mut delay = Delay;

    // Calibración: tara
    esp_println::println!("No ponga ningún objeto sobre la balanza...");
    delay.delay_ms(2000).await;
    esp_println::println!("Destarando...");

    let mut offset: i32 = 0;
    for _ in 0..20 {
        if let Ok(raw) = hx711.read() {
            offset += raw;
        }
        delay.delay_ms(50).await;
    }
    offset /= 20;
    esp_println::println!("Offset: {}", offset);
    esp_println::println!("Coloque un peso conocido y ajuste CALIBRATION_SCALE");

    // Loop de pesaje
    loop {
        if let Ok(raw_value) = hx711.read() {
            let weight = ((raw_value - offset) as f32 / CALIBRATION_SCALE)-3.68;
            esp_println::println!("Peso: {:.2} kg", weight);
        } else {
            esp_println::println!("Error al leer HX711");
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}
