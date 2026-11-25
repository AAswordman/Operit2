use super::{newTestJsEngine, testJavaScriptRuntimeHost, TestPluginConfigExecutionHost};
use operit_plugin_sdk::javascript::{ToolPkgConfigScope, ToolPkgTextResourceHost};
use operit_plugin_sdk::toolpkg::ToolPkgLoader::ToolPkgLoader;
use operit_plugin_sdk::toolpkg::ToolPkgParser::{ToolPkgArchiveParser, ToolPkgSourceType};
use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::sync::{Arc, Mutex};

/// Validates an archive through the production loader, including every reported child error.
async fn validate_archive(bytes: &[u8], path: &str, builtin: bool) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    let index = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
    let resources: BTreeMap<String, String> = index
        .entryNames
        .iter()
        .filter_map(|name| {
            ToolPkgArchiveParser::readZipEntryText(&mut archive, &index, name)
                .map(|text| (name.to_ascii_lowercase(), text))
        })
        .collect();
    let resources: Arc<dyn ToolPkgTextResourceHost> = Arc::new(resources);
    let scope =
        ToolPkgArchiveParser::readInstallationScope(bytes)?.unwrap_or(ToolPkgConfigScope::Device);
    let errors = Mutex::new(Vec::new());
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let result = ToolPkgLoader::loadToolPkgFromArchiveBytes(
        bytes,
        if builtin {
            ToolPkgSourceType::ASSET
        } else {
            ToolPkgSourceType::EXTERNAL
        },
        path,
        builtin,
        scope,
        &engine,
        resources,
        |name, error| errors.lock().unwrap().push(format!("{name}: {error}")),
    )
    .await;
    engine.destroy();
    result?;
    let errors = errors.into_inner().unwrap();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Parses every synchronized artifact, even when its activation default is disabled.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Run by sync_plugin_packages.py after generating the selected package outputs"]
async fn bundled_packages_parse() {
    testJavaScriptRuntimeHost();
    let inputs = std::env::var("OPERIT_PLUGIN_PARSE_INPUTS")
        .expect("package gate requires explicit output directories");
    let inputs: BTreeMap<String, String> =
        serde_json::from_str(&inputs).expect("package gate input JSON");
    assert!(
        !inputs.is_empty(),
        "package gate requires output directories"
    );
    let mut failures = Vec::new();
    let mut count = 0;
    for (kind, directory) in inputs {
        let mut paths = std::fs::read_dir(&directory)
            .expect("package gate output directory")
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.is_file()
                    && matches!(
                        path.extension().and_then(|value| value.to_str()),
                        Some("js" | "toolpkg")
                    )
            })
            .collect::<Vec<_>>();
        paths.sort();
        assert!(!paths.is_empty(), "No package artifacts in {directory}");
        for path in paths {
            count += 1;
            let label = format!("{kind}/{}", path.file_name().unwrap().to_string_lossy());
            let bytes = std::fs::read(&path).expect("read synchronized package");
            let result = if path.extension().unwrap() == "toolpkg" {
                validate_archive(&bytes, &path.to_string_lossy(), kind == "buildin").await
            } else {
                std::str::from_utf8(&bytes)
                    .map_err(|error| error.to_string())
                    .and_then(JsPackageLoader::parse)
                    .map(|_| ())
            };
            match result {
                Ok(()) => println!("PARSED: {label}"),
                Err(error) => failures.push(format!("{label}: {error}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Package parsing rejected {} of {count} artifacts:\n{}",
        failures.len(),
        failures.join("\n")
    );
    println!("PACKAGE-PARSE-PASSED: {count} artifacts");
}

/// Builds a disabled archive fixture without substituting any production parser behavior.
fn fixture(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, source) in entries {
        archive
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(source.as_bytes()).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

/// Rejects the exact unexported callback regression introduced by the character group-turn API.
#[tokio::test(flavor = "current_thread")]
async fn registration_requires_exported_callbacks() {
    testJavaScriptRuntimeHost();
    let manifest = r#"{"schema_version":1,"toolpkg_id":"test.gate","api_version":"2.0.0","enabled_by_default":false,"main":"main.js","public_api":"api.ts","subpackages":[]}"#;
    let source = "const groupTurnApi = async function() {}; exports.registerToolPkg = function() { ToolPkg.registerApi({name:'chat.groupTurn',function:groupTurnApi}); return true; };";
    let invalid = fixture(&[
        ("manifest.json", manifest),
        ("api.ts", "export interface Api {}"),
        ("main.js", source),
    ]);
    let error = validate_archive(&invalid, "unexported.toolpkg", true)
        .await
        .unwrap_err();
    assert!(
        error.contains("exported ToolPkg function reference"),
        "{error}"
    );
    let exported = format!("{source}\nexports.groupTurnApi = groupTurnApi;");
    let valid = fixture(&[
        ("manifest.json", manifest),
        ("api.ts", "export interface Api {}"),
        ("main.js", &exported),
    ]);
    validate_archive(&valid, "exported.toolpkg", false)
        .await
        .unwrap();
}

/// Rejects partial container loads when one child has invalid package metadata.
#[tokio::test(flavor = "current_thread")]
async fn child_parse_errors_reject_the_archive() {
    testJavaScriptRuntimeHost();
    let manifest = r#"{"schema_version":1,"toolpkg_id":"test.gate","main":"main.js","subpackages":[{"id":"good","entry":"good.js"},{"id":"bad","entry":"bad.js"}]}"#;
    let bytes = fixture(&[
        ("manifest.json", manifest),
        (
            "main.js",
            "exports.registerToolPkg = function() { return true; };",
        ),
        ("good.js", "/* METADATA {name: good} */"),
        ("bad.js", "/* METADATA {tools: []} */"),
    ]);
    let error = validate_archive(&bytes, "partial.toolpkg", false)
        .await
        .unwrap_err();
    assert!(error.contains("name"), "{error}");
}
