#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[embassy_executor::task]
async fn counter(){
    loop {
        let wait=Timer::after(Duration::from_secs(5));
        esp_println::println!("Every 5");
        wait.await;
    }
}

#[embassy_executor::task]
async fn composed_counter(){
    loop{
        esp_println::println!("I'm gonna wait 3 seconds");
        let t1=Timer::after(Duration::from_secs(3));
        esp_println::println!("First count initialized");
        let t2=Timer::after(Duration::from_secs(2));
        esp_println::println!("Second count initialized");
        embassy_futures::join::join(t1,t2).await;
        esp_println::println!("Three seconds");
    }
}

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    spawner.spawn(run().unwrap());
    spawner.spawn(counter().unwrap());
    spawner.spawn(composed_counter().unwrap());

    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
