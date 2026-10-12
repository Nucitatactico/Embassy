#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    dma_rx_buffer,
    i2s::master::{I2s, TdmConfig, DataFormat},
    i2s::master::asynch::I2sReadDmaAsync,  // <-- Ruta corregida
    time::Rate,
    timer::timg::TimerGroup,
};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_println::println!("Init!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // 1. Canal DMA
    let dma_channel = peripherals.DMA_I2S0;

    // 2. Configuración TDM Philips
    let tdm_config = TdmConfig::new_tdm_philips()
        .with_sample_rate(Rate::from_hz(44100))
        .with_data_format(DataFormat::Data16Channel16);

    // 3. Inicializar I2S
    let i2s = I2s::new(
        peripherals.I2S0,
        dma_channel,
        tdm_config,
    ).unwrap();

    // 4. Buffer DMA para RX (creado con la macro específica)
    let rx_buffer = dma_rx_buffer!(4092 * 4);

    // 5. Construir receptor
    let mut i2s_rx = i2s.i2s_rx
        .with_bclk(peripherals.GPIO33)
        .with_ws(peripherals.GPIO25)
        .with_din(peripherals.GPIO32)
        .build();

    // 6. Iniciar transferencia circular asíncrona
    let mut transfer = i2s_rx
        .read_dma_circular_async(rx_buffer)
        .await
        .unwrap();

    esp_println::println!("Iniciando captura de audio...");
    loop {
        let available = transfer.available().await.unwrap_or(0);
        if available > 0 {
            let mut chunk = [0u8; 128];
            let to_read = available.min(128);
            if transfer.pop(&mut chunk[..to_read]).await.is_ok() {
                let samples: &[i16] = unsafe {
                    core::slice::from_raw_parts(
                        chunk.as_ptr() as *const i16,
                        to_read / 2,
                    )
                };

                let mut max_abs: i16 = 0;
                for &s in samples {
                    let abs = if s < 0 { -s } else { s };
                    if abs > max_abs {
                        max_abs = abs;
                    }
                }
                esp_println::println!("Nivel: {}", max_abs);
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}
