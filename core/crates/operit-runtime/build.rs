use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MANIFEST_FILENAMES: &[&str] = &["manifest.hjson", "manifest.json"];

struct BuildinAsset {
    name: String,
    path: PathBuf,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let source_dir = manifest_dir
        .join("assets")
        .join("plugins")
        .join("buildin");
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
