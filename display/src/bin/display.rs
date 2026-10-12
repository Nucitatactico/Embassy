// src/bin/main.rs

#![no_std]
#![no_main]

mod display;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    rng::Rng,
    time::Rate,
    timer::timg::TimerGroup,
};

use display::Display;

esp_bootloader_esp_idf::esp_app_desc!();

const LCD_I2C_ADDR: u8 = 0x27;

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // I2C: SDA=GPIO21, SCL=GPIO22
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    let mut lcd = Display::new(i2c, LCD_I2C_ADDR).await;

    // Etiquetas fijas
    lcd.write_at(0, 0, "PH: ").await;
    lcd.write_at(1, 0, "T: ").await;

    let mut rng = Rng::new();

    loop {
        // pH 0.0 ..= 14.0 (décimas)
        let ph = (rng.random() % 141) as f32 / 10.0;
        // Temperatura -10.0 ..= 50.0 (décimas)
        let temp = (rng.random() % 601) as f32 / 10.0 - 10.0;

        // Formating the text whitout affecting the buffer' size
        let mut ph_buf: heapless::String<8> = heapless::String::new();
        let mut t_buf: heapless::String<8> = heapless::String::new();
        core::fmt::Write::write_fmt(&mut ph_buf, format_args!("{:.1} ", ph)).ok();
        core::fmt::Write::write_fmt(&mut t_buf, format_args!("{:.1} ", temp)).ok();

        lcd.write_at(0, 4, &ph_buf).await;
        lcd.write_at(1, 3, &t_buf).await;

        Timer::after(Duration::from_millis(1_000)).await;
    }
}
