/// Runs the stdin/stdout bridge process used by the Windows Flutter runner.
fn main() {
    if let Err(error) = operit_flutter_bridge::process_stdio::run_stdio_server() {
        eprintln!("operit flutter bridge process stopped: {error}");
        std::process::exit(1);
    }
}
