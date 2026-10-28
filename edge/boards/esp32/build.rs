/// Exports the ESP-IDF SDK environment when building the firmware target.
fn main() {
    println!("cargo:rerun-if-env-changed=OPERIT_WIFI_SSID");
    println!("cargo:rerun-if-env-changed=OPERIT_WIFI_PASSWORD");
    println!("cargo:rerun-if-env-changed=OPERIT_TIMEZONE");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }
}
