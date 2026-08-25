use std::env;
use std::path::PathBuf;

use operit_plugin_sdk_codegen::{check_declaration_tree, generate_declaration_tree};

/// Runs declaration generation or consistency checking for the Rust plugin SDK.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let command = arguments.next().ok_or("missing codegen command")?;
    let source_root = PathBuf::from(arguments.next().ok_or("missing SDK source root")?);
    let declaration_root = PathBuf::from(arguments.next().ok_or("missing declaration root")?);
    if arguments.next().is_some() {
        return Err("unexpected codegen argument".into());
    }

    match command.to_string_lossy().as_ref() {
        "generate" => {
            generate_declaration_tree(&source_root, &declaration_root)?;
        }
        "check" => {
            check_declaration_tree(&source_root, &declaration_root)?;
        }
        command => return Err(format!("unknown codegen command: {command}").into()),
    }
    Ok(())
}
