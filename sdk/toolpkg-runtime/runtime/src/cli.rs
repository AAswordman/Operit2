use std::env;
use std::path::PathBuf;

use toolpkg_runtime::loadToolPkgSnapshotJson;
use serde_json::json;

use crate::server::runToolPkgRuntimeServer;

#[allow(non_snake_case)]
pub fn runMain() {
    match run() {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
        }
        Err(error) => {
            eprintln!("{}", json!({ "success": false, "message": error }));
            std::process::exit(1);
        }
    }
}

fn run() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage().to_string())?;
    match command {
        "snapshot" => runSnapshot(&args),
        "serve" => runServe(&args),
        other => Err(format!("unknown command: {other}")),
    }
}

#[allow(non_snake_case)]
fn runSnapshot(args: &[String]) -> Result<String, String> {
    if args.len() != 4 {
        return Err(usage().to_string());
    }
    if args[2] != "--language" {
        return Err("missing --language <language-code>".to_string());
    }
    let languageCode = normalizedLanguageCode(&args[3])?;
    loadToolPkgSnapshotJson(PathBuf::from(&args[1]), &languageCode)
}

#[allow(non_snake_case)]
fn runServe(args: &[String]) -> Result<String, String> {
    if args.len() != 3 {
        return Err(usage().to_string());
    }
    if args[1] != "--language" {
        return Err("missing --language <language-code>".to_string());
    }
    let languageCode = normalizedLanguageCode(&args[2])?;
    runToolPkgRuntimeServer(languageCode)?;
    Ok(String::new())
}

#[allow(non_snake_case)]
fn normalizedLanguageCode(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("language code is required".to_string());
    }
    Ok(trimmed.to_string())
}

fn usage() -> &'static str {
    "usage: toolpkg-runtime snapshot <toolpkg-path> --language <language-code>\n       toolpkg-runtime serve --language <language-code>"
}
