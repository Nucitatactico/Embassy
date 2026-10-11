#![no_std] 
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup; 
use esp_println::println;
use embassy_futures::join::join as myMacro;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    println!("Embassy initialized!");
    spawner.spawn(counter()).unwrap();
    spawner.spawn(composed_counter()).unwrap();

    loop {
        println!("Every 10 seconds");
        Timer::after(Duration::from_secs(10)).await;
    }
}
#[embassy_executor::task]
async fn counter(){
    let wait=Timer::after(Duration::from_secs(5));
    println!("Every 5");
    println!("I'm doing whatever I want without waiting.");
    wait.await;
}
#[embassy_executor::task]
async fn composedCounter(){
    println!("I'm gonna wait 3 seconds");
    let t1=Timer::after(Duration::from_secs(3));
    println!("Nah, you're alone on this lil bro.");
    let t2=Timer::after(Duration::from_secs(2));
    println!("I'm currently waiting 5 seconds.");
    myMacro(t1,t2).await;
}
