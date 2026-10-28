#![allow(non_snake_case)]

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use operit_board_pb_sbc01_h3::PbSbc01H3FaceStateFile;

const POLL_INTERVAL_MS: u64 = 250;

/// Starts the console robot face renderer for a PB_SBC01_H3 state file.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the robot face renderer loop.
fn run() -> Result<(), String> {
    let stateFile = parseStateFile(std::env::args().skip(1))?;
    loop {
        let state = readState(&stateFile)?;
        renderState(&state)?;
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}

/// Parses the required state file argument for the face renderer.
fn parseStateFile<I>(args: I) -> Result<PathBuf, String>
where
    I: IntoIterator<Item = String>,
{
    let mut iterator = args.into_iter();
    let mut stateFile = None;
    while let Some(arg) = iterator.next() {
        match arg.as_str() {
            "--state-file" => stateFile = Some(readArgValue(&mut iterator, "--state-file")?),
            "--help" | "-h" => return Err(faceUsage()),
            _ => {
                return Err(format!(
                    "unknown face renderer argument: {arg}\n{}",
                    faceUsage()
                ))
            }
        }
    }
    stateFile
        .map(PathBuf::from)
        .ok_or_else(|| format!("--state-file is required\n{}", faceUsage()))
}

/// Reads the value following one face renderer command-line option.
fn readArgValue<I>(iterator: &mut I, name: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    iterator
        .next()
        .ok_or_else(|| format!("{name} requires a value"))
}

/// Reads one PB_SBC01_H3 face state file.
fn readState(path: &PathBuf) -> Result<PbSbc01H3FaceStateFile, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("read robot face state failed: {error}"))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("decode robot face state failed: {error}"))
}

/// Renders one robot face state to the active terminal display.
fn renderState(state: &PbSbc01H3FaceStateFile) -> Result<(), String> {
    let face = faceGlyph(&state.expression)?;
    print!("\x1B[2J\x1B[H");
    println!("+----------------------+");
    println!("|                      |");
    println!("|        {}        |", face.eyes);
    println!("|                      |");
    println!("|         {}         |", face.mouth);
    println!("|                      |");
    println!("+----------------------+");
    println!("PB_SBC01_H3  {}", state.expression);
    Ok(())
}

/// Selects a console glyph for one supported robot expression.
fn faceGlyph(expression: &str) -> Result<FaceGlyph, String> {
    match expression {
        "neutral" => Ok(FaceGlyph::new("o    o", "----")),
        "booting" => Ok(FaceGlyph::new(".    .", "....")),
        "online" => Ok(FaceGlyph::new("^    ^", "\\__/")),
        "listening" => Ok(FaceGlyph::new("O    O", "----")),
        "thinking" => Ok(FaceGlyph::new("-    o", "....")),
        "speaking" => Ok(FaceGlyph::new("o    o", "====")),
        "happy" => Ok(FaceGlyph::new("^    ^", "\\__/")),
        "sleeping" => Ok(FaceGlyph::new("-    -", "____")),
        "error" => Ok(FaceGlyph::new("x    x", "!!!!")),
        _ => Err(format!("unsupported robot face expression: {expression}")),
    }
}

/// Describes one ASCII robot face glyph.
struct FaceGlyph {
    eyes: &'static str,
    mouth: &'static str,
}

impl FaceGlyph {
    /// Creates one ASCII robot face glyph.
    fn new(eyes: &'static str, mouth: &'static str) -> Self {
        Self { eyes, mouth }
    }
}

/// Returns concise command usage for the face renderer.
fn faceUsage() -> String {
    "usage: operit-pb-sbc01-h3-face --state-file <path>".to_string()
}
