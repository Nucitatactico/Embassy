#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    i2c::master::{Config as I2cConfig, I2c},
    time::Rate,
    timer::timg::TimerGroup,
};
use hd44780_i2c_nostd::LcdI2c;

// --- Dependencias del sensor (comentadas de momento) ---
// use esp_hal::{
//     delay::Delay as HalDelay,
//     gpio::Flex,
// };
// use heapless::String;
// use onewire::{ds18b20, Device, DeviceSearch, OneWire, DS18B20};

use esp_alloc as _;

esp_bootloader_esp_idf::esp_app_desc!();

const LCD_I2C_ADDR: u8 = 0x27;

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    esp_println::println!("[BOOT] esp-rtos iniciado");

    // I2C para el LCD (SDA=GPIO21, SCL=GPIO22)
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    // --- Sensor DS18B20 (comentado de momento) ---
    // let ds_pin = Flex::new(peripherals.GPIO13);
    // spawner.spawn(temp_task(i2c, ds_pin).unwrap());

    // Micrófono (LM393 OUT) en GPIO5
    let mic_out = Input::new(
        peripherals.GPIO5,
        InputConfig::default().with_pull(Pull::Up),
    );

    // Zumbador en GPIO18
    let buzzer = Output::new(peripherals.GPIO18, Level::Low, OutputConfig::default());

    spawner.spawn(display_task(i2c).unwrap());
    spawner.spawn(mic_task(mic_out, buzzer).unwrap());
    esp_println::println!("[BOOT] tareas lanzadas");
}

// ============================================================
// TAREA: Mostrar mensaje fijo en el LCD
// ============================================================
#[embassy_executor::task]
async fn display_task(i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Async>) {
    let mut lcd = LcdI2c::new(i2c, LCD_I2C_ADDR);
    let mut lcd_delay = Delay;

    if let Err(e) = lcd.init(&mut lcd_delay).await {
        esp_println::println!("[LCD] init falló: {:?}", e);
        return;
    }
    lcd.set_backlight(true);

    lcd.set_cursor(0, 0, &mut lcd_delay).await.unwrap();
    lcd.write_str("en mantenimiento", &mut lcd_delay).await.unwrap();
    esp_println::println!("[LCD] mensaje mostrado");

    // El mensaje se queda fijo en pantalla
    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}

// ============================================================
// TAREA: Alarma por sonido (micrófono + zumbador)
// ============================================================
#[embassy_executor::task]
async fn mic_task(mic_out: Input<'static>, mut buzzer: Output<'static>) {
    loop {
        if mic_out.is_low() {
            buzzer.set_high();
            Timer::after(Duration::from_millis(100)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(200)).await;
        } else {
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

// ============================================================
// TAREA: Lectura del DS18B20 (COMENTADA de momento)
// ============================================================
// #[embassy_executor::task]
// async fn temp_task(
//     i2c: esp_hal::i2c::master::I2c<'static, esp_hal::Async>,
//     ds_pin: esp_hal::gpio::Flex<'static>,
// ) {
//     let mut hal_delay = esp_hal::delay::Delay::new();
//     let mut one_wire_bus = onewire::OneWire::new(ds_pin, false);
//
//     let mut search = onewire::DeviceSearch::new();
//     let mut sensor_device: Option<onewire::Device> = None;
//
//     while let Some(device) = one_wire_bus.search_next(&mut search, &mut hal_delay).unwrap() {
//         esp_println::println!("[1-Wire] dispositivo encontrado: {:02X?}", device.address);
//         if device.address[0] == onewire::ds18b20::FAMILY_CODE {
//             sensor_device = Some(device);
//             break;
//         }
//     }
//
//     let mut sensor = match sensor_device {
//         Some(dev) => {
//             esp_println::println!("[1-Wire] DS18B20 OK");
//             onewire::DS18B20::new(dev).unwrap()
//         }
//         None => {
//             esp_println::println!("[1-Wire] no se encontró DS18B20");
//             return;
//         }
//     };
//
//     let mut lcd = hd44780_i2c_nostd::LcdI2c::new(i2c, LCD_I2C_ADDR);
//     let mut lcd_delay = embassy_time::Delay;
//
//     if let Err(e) = lcd.init(&mut lcd_delay).await {
//         esp_println::println!("[LCD] init falló: {:?}", e);
//         return;
//     }
//     lcd.set_backlight(true);
//
//     lcd.set_cursor(0, 0, &mut lcd_delay).await.unwrap();
//     lcd.write_str("T: ", &mut lcd_delay).await.unwrap();
//     esp_println::println!("[LCD] inicializado");
//
//     loop {
//         let resolution = sensor
//             .measure_temperature(&mut one_wire_bus, &mut hal_delay)
//             .unwrap();
//
//         embassy_time::Timer::after_millis(resolution.time_ms() as u64).await;
//
//         match sensor.read_temperature(&mut one_wire_bus, &mut hal_delay) {
//             Ok(temp_raw) => {
//                 let temp_c = temp_raw as f32 / 16.0;
//                 let mut buf: heapless::String<8> = heapless::String::new();
//                 core::fmt::Write::write_fmt(&mut buf, format_args!("{:.1} ", temp_c)).ok();
//
//                 lcd.set_cursor(0, 3, &mut lcd_delay).await.unwrap();
//                 lcd.write_str(&buf, &mut lcd_delay).await.unwrap();
//                 esp_println::println!("[Temp] {:.1} C", temp_c);
//             }
//             Err(e) => {
//                 esp_println::println!("[Temp] error: {:?}", e);
//             }
//         }
//
//         embassy_time::Timer::after(embassy_time::Duration::from_millis(1000)).await;
//     }
// }
