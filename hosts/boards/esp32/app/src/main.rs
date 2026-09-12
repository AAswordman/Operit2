use std::error::Error;
use std::sync::Arc;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::peripherals::Peripherals;
use log::info;
use operit_board_esp32::{createRuntimeHostManager, Esp32GpioHost};
use operit_node_edge::EdgeNode;
use operit_proxy_edge::EdgeProxy;

mod app;

use app::Esp32App;

const LED_PIN: u8 = 2;

/// Starts the ESP32 app shell and runs the typed application loop.
fn main() -> Result<(), Box<dyn Error>> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let gpioHost = Arc::new(Esp32GpioHost::new(LED_PIN, peripherals.pins.gpio2)?);
    let hostManager = createRuntimeHostManager(gpioHost);
    let edgeNode = EdgeNode::fromHostManager(hostManager);
    let edgeProxy = EdgeProxy::new(edgeNode);
    let mut app = Esp32App::new(edgeProxy, LED_PIN);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    loop {
        let state = runtime.block_on(app.tick())?;
        info!(
            "Operit ESP32 LED state: pin={} level={}",
            state.pin, state.level
        );
        FreeRtos::delay_ms(1000);
    }
}
