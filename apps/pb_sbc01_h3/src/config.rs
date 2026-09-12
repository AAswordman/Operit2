#![allow(non_snake_case)]

use std::env;
use std::path::PathBuf;

const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:4819";

/// Carries explicit runtime settings for the PB_SBC01_H3 controller.
#[derive(Clone, Debug)]
pub struct PbSbc01H3Config {
    pub bindAddress: String,
    pub token: String,
    pub runtimeRoot: PathBuf,
    pub workspaceRoot: PathBuf,
    pub stateRoot: PathBuf,
}

impl PbSbc01H3Config {
    /// Parses PB_SBC01_H3 controller settings from command-line arguments.
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let dataRoot = defaultDataRoot()?;
        let mut bindAddress = DEFAULT_BIND_ADDRESS.to_string();
        let mut token = String::new();
        let mut runtimeRoot = dataRoot.join("runtime");
        let mut workspaceRoot = dataRoot.join("workspace");
        let mut stateRoot = dataRoot.join("state");
        let mut iterator = args.into_iter();

        while let Some(arg) = iterator.next() {
            match arg.as_str() {
                "--bind" => bindAddress = readArgValue(&mut iterator, "--bind")?,
                "--token" => token = readArgValue(&mut iterator, "--token")?,
                "--runtime-root" => {
                    runtimeRoot = PathBuf::from(readArgValue(&mut iterator, "--runtime-root")?)
                }
                "--workspace-root" => {
                    workspaceRoot = PathBuf::from(readArgValue(&mut iterator, "--workspace-root")?)
                }
                "--state-root" => {
                    stateRoot = PathBuf::from(readArgValue(&mut iterator, "--state-root")?)
                }
                "--help" | "-h" => return Err(usage()),
                _ => return Err(format!("unknown PB_SBC01_H3 argument: {arg}\n{}", usage())),
            }
        }

        if token.trim().is_empty() {
            return Err(format!("--token is required\n{}", usage()));
        }

        Ok(Self {
            bindAddress,
            token,
            runtimeRoot,
            workspaceRoot,
            stateRoot,
        })
    }
}

/// Reads the value following one command-line option.
fn readArgValue<I>(iterator: &mut I, name: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    iterator
        .next()
        .ok_or_else(|| format!("{name} requires a value"))
}

/// Returns the PB_SBC01_H3 data root under the current Linux user home directory.
fn defaultDataRoot() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .ok_or_else(|| "HOME is required for PB_SBC01_H3 data directories".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("operit2")
        .join("pb_sbc01_h3"))
}

/// Returns concise command usage for the PB_SBC01_H3 controller.
fn usage() -> String {
    [
        "usage: operit-pb-sbc01-h3 --token <token> [--bind <address:port>]",
        "       [--runtime-root <path>] [--workspace-root <path>] [--state-root <path>]",
    ]
    .join("\n")
}
