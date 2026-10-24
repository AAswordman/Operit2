//! Offline OCR and PDF rasterization provided by the packaged Alpine tools.

use std::fs;
use std::path::Path;

use operit_host_api::{HostError, HostResult, OCRLanguage, OCRQuality, TerminalHost};

use crate::terminal::VROOT_HOST_MOUNT_ROOT;

/// Quotes an exact argument for the guest POSIX shell.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

/// Converts a physical app file to its canonical guest mount location.
fn guestPath(path: &Path) -> HostResult<String> {
    let absolute = fs::canonicalize(path)?;
    Ok(format!("{VROOT_HOST_MOUNT_ROOT}{}", absolute.display()))
}

/// Executes a bounded tool invocation and preserves its actual failure output.
fn runTool(terminal: &dyn TerminalHost, command: &str) -> HostResult<()> {
    let result = terminal.executeHiddenCommand(command, "ohos-documents", 120_000)?;
    if result.timedOut || result.exitCode != 0 {
        return Err(HostError::new(format!(
            "OpenHarmony document tool failed (exit={}, timedOut={}): {}",
            result.exitCode, result.timedOut, result.output
        )));
    }
    Ok(())
}

/// Recognizes the requested script offline with the packaged Tesseract language data.
pub fn recognizeText(
    terminal: &dyn TerminalHost,
    runtimeRoot: &Path,
    imagePath: &str,
    language: OCRLanguage,
    quality: OCRQuality,
) -> HostResult<String> {
    let temporary = tempfile::Builder::new()
        .prefix("ohos-ocr-")
        .tempdir_in(runtimeRoot)?;
    let output = temporary.path().join("text");
    let languages = match language {
        OCRLanguage::Latin => "eng",
        OCRLanguage::Chinese => "chi_sim+chi_tra+eng",
        OCRLanguage::Japanese => "jpn+eng",
        OCRLanguage::Korean => "kor+eng",
    };
    // High quality enables multiple script models; low quality uses the primary script only.
    let languages = match quality {
        OCRQuality::High => languages,
        OCRQuality::Low => match language {
            OCRLanguage::Latin => "eng",
            OCRLanguage::Chinese => "chi_sim",
            OCRLanguage::Japanese => "jpn",
            OCRLanguage::Korean => "kor",
        },
    };
    let command = format!(
        "OMP_THREAD_LIMIT=2 /usr/bin/tesseract {} {} -l {} --psm 3",
        quote(&guestPath(Path::new(imagePath))?),
        quote(&format!("{VROOT_HOST_MOUNT_ROOT}{}", output.display())),
        quote(languages),
    );
    runTool(terminal, &command)?;
    fs::read_to_string(output.with_extension("txt")).map_err(Into::into)
}

const PDF_SCRIPT: &str = r#"
import json, os, subprocess, sys
source, destination, pages_json, scale_text = sys.argv[1:]
pages = json.loads(pages_json)
dpi = str(float(scale_text) * 72)
if pages is None:
    subprocess.run(['/usr/bin/pdftoppm', '-png', '-r', dpi, source, os.path.join(destination, 'page')], check=True)
    paths = sorted((name for name in os.listdir(destination) if name.endswith('.png')), key=lambda name: int(name[5:-4]))
else:
    paths = []
    for index, page in enumerate(pages):
        if type(page) is not int or page < 0:
            raise ValueError('Invalid PDF page index')
        prefix = os.path.join(destination, 'page-' + str(index))
        subprocess.run(['/usr/bin/pdftoppm', '-png', '-r', dpi, '-f', str(page + 1), '-l', str(page + 1), '-singlefile', source, prefix], check=True)
        paths.append(os.path.basename(prefix) + '.png')
with open(os.path.join(destination, 'pages.json'), 'w', encoding='utf-8') as output:
    json.dump(paths, output)
"#;

/// Rasterizes the requested PDF pages and returns their ordered PNG names.
pub fn renderPdf(
    terminal: &dyn TerminalHost,
    input: &Path,
    outputDirectory: &Path,
    pages: Option<Vec<u32>>,
    scale: f64,
) -> HostResult<Vec<String>> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(HostError::new("PDF scale must be finite and positive"));
    }
    let command = format!(
        "/usr/bin/python3 -c {} {} {} {} {}",
        quote(PDF_SCRIPT),
        quote(&guestPath(input)?),
        quote(&guestPath(outputDirectory)?),
        quote(&serde_json::to_string(&pages).map_err(|error| HostError::new(error.to_string()))?),
        quote(&scale.to_string()),
    );
    runTool(terminal, &command)?;
    serde_json::from_slice(&fs::read(outputDirectory.join("pages.json"))?)
        .map_err(|error| HostError::new(format!("Invalid rendered PDF manifest: {error}")))
}
