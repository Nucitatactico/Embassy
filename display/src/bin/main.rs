#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, Delay};
use esp_backtrace as _;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    rng::Rng,
    timer::timg::TimerGroup,
    time::Rate,
};
use hd44780_i2c_nostd::LcdI2c;

esp_bootloader_esp_idf::esp_app_desc!();

const LCD_I2C_ADDR: u8 = 0x27;

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // I2C para el LCD
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    let mut lcd = LcdI2c::new(i2c, LCD_I2C_ADDR);
    let mut delay = Delay;

    lcd.init(&mut delay).await.unwrap();
    lcd.set_backlight(true);

    // Etiquetas fijas, se escriben una sola vez
    lcd.set_cursor(0, 0, &mut delay).await.unwrap();
    lcd.write_str("PH: ", &mut delay).await.unwrap();

    lcd.set_cursor(1, 0, &mut delay).await.unwrap();
    lcd.write_str("T: ", &mut delay).await.unwrap();

    // RNG del ESP32
    let mut rng = Rng::new();

    loop {
        // pH entre 0.0 y 14.0
        let ph_raw = rng.random() % 141;   // 0..=140
        let ph = ph_raw as f32 / 10.0;

        // Temperatura entre -10.0 y 50.0
        let temp_raw = rng.random() % 601; // 0..=600
        let temp = temp_raw as f32 / 10.0 - 10.0;

        // Formateamos a ancho fijo con padding de espacios para borrar restos.
        // Usamos heapless::String para no depender de alloc.
        let mut ph_buf: heapless::String<8> = heapless::String::new();
        let mut t_buf: heapless::String<8> = heapless::String::new();

        // pH: 4 caracteres + espacio → "00.0 " o "14.0 "
        core::fmt::Write::write_fmt(&mut ph_buf, format_args!("{:.1} ", ph)).ok();
        // T:  5 caracteres + espacio → "-10.0 " o " 50.0 "
        core::fmt::Write::write_fmt(&mut t_buf, format_args!("{:.1} ", temp)).ok();

        lcd.set_cursor(0, 4, &mut delay).await.unwrap();
        lcd.write_str(&ph_buf, &mut delay).await.unwrap();

        lcd.set_cursor(1, 3, &mut delay).await.unwrap();
        lcd.write_str(&t_buf, &mut delay).await.unwrap();

        esp_println::println!("pH={:.1} T={:.1}", ph, temp);

        Timer::after(Duration::from_millis(1_000)).await;
    }
}
