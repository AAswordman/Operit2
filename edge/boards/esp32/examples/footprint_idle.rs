//! Build-only reference for the ESP-IDF/Rust startup cost.
//! This is a size probe, not an Edge node. Do not flash it over a paired device.

fn main() {
    esp_idf_svc::sys::link_patches();
    loop { esp_idf_hal::delay::FreeRtos::delay_ms(1000); }
}
