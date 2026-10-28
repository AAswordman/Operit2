/// Exports the ESP-IDF SDK environment to the Rust build.
fn main() {
    embuild::espidf::sysenv::output();
}
