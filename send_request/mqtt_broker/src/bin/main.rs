#![no_std]
#![no_main]

mod display;

use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources, Runner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    i2c::master::{Config as I2cConfig, I2c},
    interrupt::software::SoftwareInterruptControl,
    time::Rate,
    timer::timg::TimerGroup,
};
// Imports de esp-radio 0.18 para modo AP
use esp_radio::wifi::{
    Config as WifiConfig, AccessPointConfig, Configuration, WifiController,
    WifiDevice, Interface,
};
// Broker MQTT embebido
use gatomqtt::{
    handler::connection::connection_loop,
    router::RetainedStore,
    session::registry::SessionRegistry,
};
use embassy_sync::mutex::Mutex;
use display::Display;

esp_bootloader_esp_idf::esp_app_desc!();

// --- Configuración del Access Point ---
const AP_SSID: &str = "ESP32_BROKER";
const AP_PASSWORD: &str = "password123"; // Mínimo 8 caracteres
const STATIC_IP: &str = "192.168.4.1";
const TOPIC: &str = "sensores/datos";
const LCD_I2C_ADDR: u8 = 0x27;

// Heap para esp-radio
esp_alloc::heap_allocator!(size: 72 * 1024);

// Canal para comunicar el broker con la tarea del display
static DATOS_CH: Channel<CriticalSectionRawMutex, (f32, f32), 4> = Channel::new();
static mut STACK_RESOURCES: StackResources<4> = StackResources::<4>::new();

// Estado estático del broker MQTT (bounded, sin heap)
const MAX_SESSIONS: usize = 4;
const MAX_SUBS: usize = 8;
const MAX_INFLIGHT: usize = 2;
const MAX_RETAINED: usize = 8;
const INBOUND_N: usize = 4;

static REGISTRY: Mutex<
    CriticalSectionRawMutex,
    SessionRegistry<MAX_SESSIONS, MAX_SUBS, MAX_INFLIGHT>,
> = Mutex::new(SessionRegistry::new());

static RETAINED: Mutex<CriticalSectionRawMutex, RetainedStore<MAX_RETAINED>> =
    Mutex::new(RetainedStore::new());

static INBOUND: Channel<CriticalSectionRawMutex, gatomqtt::handler::command::MqttIntent, INBOUND_N> =
    Channel::new();

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // 1. Iniciar esp-rtos ANTES que el radio
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // 2. I2C para el LCD
    let i2c = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO21)
    .with_scl(peripherals.GPIO22)
    .into_async();

    let lcd = Display::new(i2c, LCD_I2C_ADDR).await;

    // 3. Interfaz de Access Point (singleton)
    let wifi_interface = Interface::access_point();

    // 4. Controlador WiFi en modo AP
    let mut wifi_controller = WifiController::new(
        peripherals.WIFI,
        WifiConfig::AccessPoint(Default::default()),
    ).unwrap();

    // 5. Configurar IP estática para el AP
    let static_ip = embassy_net::Ipv4Cidr::new(STATIC_IP.parse().unwrap(), 24);
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: static_ip,
        gateway: Some(STATIC_IP.parse().unwrap()),
        dns_servers: Default::default(),
    });
    let resources = unsafe { &mut *core::ptr::addr_of_mut!(STACK_RESOURCES) };
    let (stack, runner) = embassy_net::new(wifi_interface, config, resources, 0x1234_5678);

    // 6. Lanzar tareas
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(wifi_ap_task(wifi_controller).unwrap());
    spawner.spawn(broker_task(stack).unwrap());
    spawner.spawn(display_task(lcd).unwrap());

    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn wifi_ap_task(mut controller: WifiController<'static>) {
    let ap_config = Configuration::AccessPoint(AccessPointConfig {
        ssid: AP_SSID.try_into().unwrap(),
        password: AP_PASSWORD.try_into().unwrap(),
        ..Default::default()
    });
    controller.set_configuration(&ap_config).unwrap();
    controller.start().await.unwrap();
    esp_println::println!("AP '{}' iniciado en {}", AP_SSID, STATIC_IP);

    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
async fn broker_task(stack: Stack<'static>) {
    let sender = DATOS_CH.sender();

    loop {
        // Aceptar conexión TCP entrante
        let mut socket = match stack.accept(1883).await {
            Ok(s) => s,
            Err(_) => {
                Timer::after(Duration::from_millis(100)).await;
                continue;
            }
        };

        esp_println::println!("Cliente MQTT conectado");

        // Ejecutar el bucle de conexión del broker para este socket
        // La API exacta puede variar; consulta `cargo doc -p gatomqtt`
        connection_loop(
            &mut socket,
            &REGISTRY,
            &RETAINED,
            &INBOUND,
        ).await;

        esp_println::println!("Cliente MQTT desconectado");
    }
}

#[embassy_executor::task]
async fn display_task(mut lcd: Display<...>) {
    // Etiquetas fijas
    lcd.write_at(0, 0, "PH: ").await;
    lcd.write_at(1, 0, "T: ").await;

    let receiver = DATOS_CH.receiver();
    loop {
        let (ph, temp) = receiver.receive().await;

        let mut ph_buf: heapless::String<8> = heapless::String::new();
        let mut t_buf: heapless::String<8> = heapless::String::new();
        core::fmt::Write::write_fmt(&mut ph_buf, format_args!("{:.1} ", ph)).ok();
        core::fmt::Write::write_fmt(&mut t_buf, format_args!("{:.1} ", temp)).ok();

        lcd.write_at(0, 4, &ph_buf).await;
        lcd.write_at(1, 3, &t_buf).await;
    }
}
