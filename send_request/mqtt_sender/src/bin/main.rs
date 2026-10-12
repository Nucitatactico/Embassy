#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources, Runner};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    interrupt::software::SoftwareInterruptControl,
    rng::Rng,
    timer::timg::TimerGroup,
};
// Imports nuevos de esp-radio
use esp_radio::wifi::{
    Config as WifiConfig, ClientConfiguration, Configuration, WifiController,
    WifiDevice, Interface,
};
use rust_mqtt::{
    client::client::MqttClient,
    client::options::ClientOptions,
    packet::v5::publish_packet::QualityOfService,
};
use core::str::FromStr;

esp_bootloader_esp_idf::esp_app_desc!();

// Configuración de red (ajusta estos valores)
const SSID: &str = "TU_WIFI_SSID";
const PASSWORD: &str = "TU_WIFI_PASSWORD";
const BROKER_IP: &str = "192.168.4.1"; // IP del ESP32 Broker (AP)
const BROKER_PORT: u16 = 1883;
const TOPIC: &str = "sensores/datos";

// Necesario para esp-radio: asignador de memoria dinámica
esp_alloc::heap_allocator!(size: 72 * 1024);

static mut STACK_RESOURCES: StackResources<3> = StackResources::<3>::new();

#[esp_hal::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Iniciar el planificador de esp-rtos ANTES que el radio
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // 1. Obtener la interfaz de estación (singleton) ANTES de crear el controlador
    let wifi_interface = Interface::station();

    // 2. Crear el controlador de WiFi (consume el periférico WIFI)
    //    Nota: `WifiController::new` ya no devuelve una tupla con interfaces.
    let mut wifi_controller = WifiController::new(
        peripherals.WIFI,
        WifiConfig::Station(Default::default()),
    ).unwrap();

    // 3. Configurar la pila de red con DHCP
    let config = Config::dhcpv4(Default::default());
    let resources = unsafe { &mut *core::ptr::addr_of_mut!(STACK_RESOURCES) };
    let (stack, runner) = embassy_net::new(wifi_interface, config, resources, 0x8765_4321);

    // 4. Lanzar tareas
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(wifi_task(wifi_controller).unwrap());
    spawner.spawn(mqtt_publisher_task(stack).unwrap());

    loop {
        Timer::after(Duration::from_secs(60)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn wifi_task(mut controller: WifiController<'static>) {
    loop {
        // La API async de esp-radio 0.18 usa `.connect()` y eventos.
        // Este es un bucle simplificado; consulta `wait_for_station_connected_event_async`
        // en la documentación para la gestión robusta de eventos.
        let client_config = Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            password: PASSWORD.try_into().unwrap(),
            ..Default::default()
        });
        controller.set_configuration(&client_config).unwrap();
        controller.connect().await.unwrap();
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn mqtt_publisher_task(stack: Stack<'static>) {
    let mut rng = Rng::new();
    let mut rx_buffer = [0u8; 1024];
    let mut tx_buffer = [0u8; 1024];

    loop {
        if !stack.is_config_up() {
            Timer::after(Duration::from_secs(2)).await;
            continue;
        }

        let remote = (BROKER_IP, BROKER_PORT);
        let mut socket = match stack.connect(remote).await {
            Ok(s) => s,
            Err(_) => {
                Timer::after(Duration::from_secs(5)).await;
                continue;
            }
        };

        let mut client = MqttClient::new(&mut socket, &mut tx_buffer, &mut rx_buffer);
        let mut options = ClientOptions::new();
        options.set_client_id("esp32_sender".as_bytes());
        options.set_keep_alive(Duration::from_secs(30));

        if client.connect(&mut options).await.is_err() {
            Timer::after(Duration::from_secs(5)).await;
            continue;
        }

        loop {
            let ph = (rng.random() % 141) as f32 / 10.0;
            let temp = (rng.random() % 601) as f32 / 10.0 - 10.0;

            let mut payload: heapless::String<32> = heapless::String::new();
            core::fmt::write(&mut payload, format_args!("{{\"ph\":{:.1},\"temp\":{:.1}}}", ph, temp)).ok();

            let result = client
                .publish(TOPIC, payload.as_bytes(), QualityOfService::AtMostOnce)
                .await;

            if result.is_err() {
                break;
            }

            esp_println::println!("Enviado: {}", payload);
            Timer::after(Duration::from_secs(1)).await;
        }

        Timer::after(Duration::from_secs(2)).await;
    }
}
