#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    timer::timg::TimerGroup,
};
use esp_backtrace as _;

esp_bootloader_esp_idf::esp_app_desc!();  // ← Debe estar aquí, a nivel global

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let sound_out = Input::new(
        peripherals.GPIO4,
        InputConfig::default().with_pull(Pull::Up),
    );

    let mut buzzer = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());

    loop {
        if sound_out.is_low() {
            buzzer.set_high();
            Timer::after(Duration::from_millis(100)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(200)).await;
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}
