#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    time::Rate,
    timer::timg::TimerGroup,
};
use embassy_bmp280::{Bmp280, Bmp280Address, Bmp280Config, PowerMode};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // I2C: SDA=GPIO21, SCL=GPIO22
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(100)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    // Inicializar con dirección 0x76
    let mut bmp = Bmp280::new(
        i2c,
        Bmp280Address::Default, // 0x76
        Bmp280Config::default()
    ).await.unwrap();

    // Modo normal (medición continua)
    bmp.set_mode(PowerMode::Normal).await.unwrap();

    esp_println::println!("Iniciando lecturas...");
    loop {
        match bmp.read().await {
            Ok(data) => {
                let temp = data.temperature_celsius();
                let press = data.pressure_hpa();
                esp_println::println!("T: {:.2} °C, P: {:.2} hPa", temp, press);
            }
            Err(_) => {
                esp_println::println!("Error de lectura");
            }
        }
        Timer::after(Duration::from_secs(2)).await;
    }
}
