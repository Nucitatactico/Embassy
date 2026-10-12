#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::semaphore::{GreedySemaphore, Semaphore};

esp_bootloader_esp_idf::esp_app_desc!();

const TIMES: u64=4;
static AUX_DONE: GreedySemaphore<CriticalSectionRawMutex> = GreedySemaphore::new(0);

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    spawner.spawn(blinking_semaphore(spawner).unwrap());
    for i in 1..=TIMES{
        spawner.spawn(multi_task(i*3).unwrap());
    }
    loop {
        esp_println::println!("Bing!");
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
#[embassy_executor::task]
async fn blinking_semaphore(caller: Spawner) {
    loop {
        esp_println::println!("Blink");
        for i in 1..=TIMES {
            caller.spawn(aux(i * 2).unwrap());
        }
        //Waiting the four allows
        for _ in 0..TIMES {
            AUX_DONE.acquire(1).await.unwrap().disarm();
        }
    }
}
#[embassy_executor::task(pool_size = 4)]
async fn aux(time: u64){
    Timer::after(Duration::from_secs(time)).await;
    esp_println::println!("This is the iteration n:{}",time/2);
    AUX_DONE.release(1);
}
#[embassy_executor::task(pool_size=4)]
async fn multi_task(time: u64){
    loop{
        esp_println::println!("Begining n:{}",time/3);
        Timer::after(Duration::from_secs(time)).await;
        esp_println::println!("Ending n:{}",time/3);
    }
}
