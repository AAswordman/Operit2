use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Deserialize;

const MANIFEST_FILENAMES: &[&str] = &["manifest.hjson", "manifest.json"];
const SKILL_FILENAMES: &[&str] = &["SKILL.md", "skill.md"];
const SKILL_INCLUDE_FILE: &str = "skill.include.json";

struct BuildinAsset {
    name: String,
    path: PathBuf,
}

#[derive(Deserialize)]
struct SkillIncludeConfig {
    includes: Vec<SkillIncludeEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillIncludeEntry {
    source: String,
    target: String,
    #[serde(default)]
    exclude_names: Vec<String>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source_dir = manifest_dir.join("assets").join("plugins").join("buildin");
    println!("cargo:rerun-if-changed={}", source_dir.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let packaged_dir = out_dir.join("buildin_plugins");
    fs::create_dir_all(&packaged_dir).expect("create buildin plugin output directory");

    let mut assets = Vec::new();
    if source_dir.is_dir() {
        let mut entries = fs::read_dir(&source_dir)
            .expect("read assets/plugins/buildin")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        entries.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
        for path in entries {
            if path.is_file() && is_syncable_file(&path) {
                let name = path
                    .file_name()
                    .expect("buildin plugin file name")
                    .to_string_lossy()
                    .to_string();
                assets.push(BuildinAsset { name, path });
                continue;
            }
            if path.is_dir() && has_manifest(&path) {
                let name = format!(
                    "{}.toolpkg",
                    path.file_name()
                        .expect("buildin plugin directory name")
                        .to_string_lossy()
                );
                let destination = packaged_dir.join(&name);
                pack_toolpkg_folder(&path, &destination);
                assets.push(BuildinAsset {
                    name,
                    path: destination,
                });
            }
        }
    }

    let generated = out_dir.join("builtin_plugin_assets.rs");
    let mut code = String::new();
    code.push_str("#[derive(Clone, Copy)]\n");
    code.push_str("pub struct BuiltinPluginAsset {\n");
    code.push_str("    pub name: &'static str,\n");
    code.push_str("    pub bytes: &'static [u8],\n");
    code.push_str("}\n\n");
    code.push_str("pub static BUILTIN_PLUGIN_ASSETS: &[BuiltinPluginAsset] = &[\n");
    for asset in assets {
        code.push_str(&format!(
            "    BuiltinPluginAsset {{ name: {:?}, bytes: include_bytes!({:?}) }},\n",
            asset.name,
            asset.path.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    fs::write(generated, code).expect("write builtin plugin asset list");

    generate_workspace_template_assets(&manifest_dir, &out_dir);
    generate_bundled_external_skill_assets(&manifest_dir, &out_dir);
}

fn is_syncable_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("js") | Some("hjson") | Some("toolpkg")
    )
}

fn has_manifest(folder: &Path) -> bool {
    MANIFEST_FILENAMES
        .iter()
        .any(|file_name| folder.join(file_name).is_file())
}

fn pack_toolpkg_folder(source_folder: &Path, destination_file: &Path) {
    let file = fs::File::create(destination_file).expect("create buildin toolpkg archive");
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut files = Vec::new();
    collect_files(source_folder, source_folder, &mut files);
    files.sort_by_key(|(_, relative)| relative.clone());
    for (file_path, relative_path) in files {
        archive
            .start_file(relative_path.replace('\\', "/"), options)
            .expect("start buildin toolpkg archive file");
        let mut source = fs::File::open(file_path).expect("open buildin plugin source file");
        let mut buffer = Vec::new();
        source
            .read_to_end(&mut buffer)
            .expect("read buildin plugin source file");
        archive
            .write_all(&buffer)
            .expect("write buildin toolpkg archive file");
    }
    archive.finish().expect("finish buildin toolpkg archive");
}

fn collect_files(base: &Path, current: &Path, files: &mut Vec<(PathBuf, String)>) {
    let mut entries = fs::read_dir(current)
        .expect("read buildin plugin directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
    for path in entries {
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("node_modules") {
                continue;
            }
            collect_files(base, &path, files);
            continue;
        }
        if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .expect("buildin plugin file must be under source folder")
                .to_string_lossy()
                .to_string();
            files.push((path, relative));
        }
    }
}

fn generate_workspace_template_assets(manifest_dir: &Path, out_dir: &Path) {
    let source_dir = manifest_dir.join("assets").join("workspace_templates");
    println!("cargo:rerun-if-changed={}", source_dir.display());

    let mut files = Vec::new();
    if source_dir.is_dir() {
        collect_files(&source_dir, &source_dir, &mut files);
    }
    files.sort_by_key(|(_, relative)| relative.clone());

    let generated = out_dir.join("workspace_template_assets.rs");
    let mut code = String::new();
    code.push_str("#[derive(Clone, Copy)]\n");
    code.push_str("pub struct WorkspaceTemplateAsset {\n");
    code.push_str("    pub path: &'static str,\n");
    code.push_str("    pub bytes: &'static [u8],\n");
    code.push_str("}\n\n");
    code.push_str("pub static WORKSPACE_TEMPLATE_ASSETS: &[WorkspaceTemplateAsset] = &[\n");
    for (path, relative) in files {
        code.push_str(&format!(
            "    WorkspaceTemplateAsset {{ path: {:?}, bytes: include_bytes!({:?}) }},\n",
            relative.replace('\\', "/"),
            path.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    fs::write(generated, code).expect("write workspace template asset list");
}

fn generate_bundled_external_skill_assets(manifest_dir: &Path, out_dir: &Path) {
    let plugins_dir = pluginsSourceRoot(manifest_dir);
    let source_dir = plugins_dir.join("skills").join("external");
    println!("cargo:rerun-if-changed={}", source_dir.display());

    let mut assets = Vec::<(String, PathBuf, String)>::new();
    if source_dir.is_dir() {
        let mut skill_dirs = fs::read_dir(&source_dir)
            .expect("read plugins/skills/external")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter(|path| has_skill_file(path))
            .collect::<Vec<_>>();
        skill_dirs.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
        for skill_dir in skill_dirs {
            let skill_name = skill_dir
                .file_name()
                .expect("bundled skill directory name")
                .to_string_lossy()
                .to_string();
            let mut files = Vec::new();
            collect_skill_source_files(&skill_dir, &skill_dir, &mut files);
            files.sort_by_key(|(_, relative)| relative.clone());
            for (path, relative) in files {
                assets.push((skill_name.clone(), path, relative.replace('\\', "/")));
            }
            collect_skill_includes(&plugins_dir, &skill_dir, &skill_name, &mut assets);
        }
    }

    let generated = out_dir.join("bundled_external_skill_assets.rs");
    let mut code = String::new();
    code.push_str("#[derive(Clone, Copy)]\n");
    code.push_str("pub struct BundledExternalSkillAsset {\n");
    code.push_str("    pub skill_name: &'static str,\n");
    code.push_str("    pub path: &'static str,\n");
    code.push_str("    pub bytes: &'static [u8],\n");
    code.push_str("}\n\n");
    code.push_str("pub static BUNDLED_EXTERNAL_SKILL_ASSETS: &[BundledExternalSkillAsset] = &[\n");
    for (skill_name, path, relative) in assets {
        code.push_str(&format!(
            "    BundledExternalSkillAsset {{ skill_name: {:?}, path: {:?}, bytes: include_bytes!({:?}) }},\n",
            skill_name,
            relative,
            path.to_string_lossy()
        ));
    }
    code.push_str("];\n");
    fs::write(generated, code).expect("write bundled external skill asset list");
}

fn has_skill_file(folder: &Path) -> bool {
    SKILL_FILENAMES
        .iter()
        .any(|file_name| folder.join(file_name).is_file())
}

fn collect_skill_source_files(base: &Path, current: &Path, files: &mut Vec<(PathBuf, String)>) {
    let mut entries = fs::read_dir(current)
        .expect("read skill source directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
    for path in entries {
        let file_name = path.file_name().and_then(|name| name.to_str());
        if path.is_dir() {
            if file_name == Some("node_modules") {
                continue;
            }
            collect_skill_source_files(base, &path, files);
            continue;
        }
        if path.is_file() {
            if file_name == Some(SKILL_INCLUDE_FILE) {
                continue;
            }
            let relative = path
                .strip_prefix(base)
                .expect("skill source file must be under source folder")
                .to_string_lossy()
                .to_string();
            files.push((path, relative));
        }
    }
}

fn collect_skill_includes(
    plugins_dir: &Path,
    skill_dir: &Path,
    skill_name: &str,
    assets: &mut Vec<(String, PathBuf, String)>,
) {
    let include_file = skill_dir.join(SKILL_INCLUDE_FILE);
    if !include_file.is_file() {
        return;
    }
    println!("cargo:rerun-if-changed={}", include_file.display());
    let config_text = fs::read_to_string(&include_file).expect("read skill include config");
    let config: SkillIncludeConfig =
        serde_json::from_str(&config_text).expect("parse skill include config");
    let plugins_canonical = plugins_dir
        .canonicalize()
        .expect("canonicalize plugins directory");

    for include in config.includes {
        let source = skill_dir
            .join(&include.source)
            .canonicalize()
            .expect("canonicalize skill include source");
        assert!(
            is_path_inside(&source, &plugins_canonical),
            "skill include source must stay under plugins directory: {}",
            source.display()
        );
        println!("cargo:rerun-if-changed={}", source.display());
        let target = normalize_asset_relative_path(&include.target);
        if source.is_file() {
            assets.push((
                skill_name.to_string(),
                source,
                path_to_asset_string(&target),
            ));
            continue;
        }

        assert!(
            source.is_dir(),
            "skill include source must be a file or directory: {}",
            source.display()
        );
        let mut include_files = Vec::new();
        collect_include_files(&source, &source, &include.exclude_names, &mut include_files);
        include_files.sort_by_key(|(_, relative)| relative.clone());
        for (path, relative) in include_files {
            assets.push((
                skill_name.to_string(),
                path,
                path_to_asset_string(&target.join(relative)),
            ));
        }
    }
}

fn collect_include_files(
    base: &Path,
    current: &Path,
    exclude_names: &[String],
    files: &mut Vec<(PathBuf, String)>,
) {
    let mut entries = fs::read_dir(current)
        .expect("read included directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
    for path in entries {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("included path file name")
            .to_string();
        if exclude_names.iter().any(|excluded| excluded == &file_name) {
            continue;
        }
        if path.is_dir() {
            collect_include_files(base, &path, exclude_names, files);
            continue;
        }
        if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .expect("included file must be under source folder")
                .to_string_lossy()
                .to_string();
            files.push((path, relative));
        }
    }
}

fn normalize_asset_relative_path(path: &str) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => normalized.push(part),
            _ => panic!("invalid skill include target path: {}", path),
        }
    }
    assert!(
        !normalized.as_os_str().is_empty(),
        "skill include target path must not be empty"
    );
    normalized
}

fn path_to_asset_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_path_inside(path: &Path, base: &Path) -> bool {
    path == base || path.starts_with(base)
}

#[allow(non_snake_case)]
fn pluginsSourceRoot(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .join("plugins")
}
