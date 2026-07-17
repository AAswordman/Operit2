use std::path::{Path, PathBuf};
use std::sync::Arc;

use operit_host_api::FileSystemHost;
use regex::Regex;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectType {
    PYTHON,
    TYPESCRIPT,
    NODEJS,
    UNKNOWN,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectStructure {
    pub r#type: ProjectType,
    pub hasRequirementsTxt: bool,
    pub hasPyprojectToml: bool,
    pub hasSetupPy: bool,
    pub hasPackageJson: bool,
    pub hasTsConfig: bool,
    pub mainPythonModule: Option<String>,
    pub mainJsFile: Option<String>,
    pub mainTsFile: Option<String>,
    pub hasTsFiles: bool,
    pub configExample: Option<String>,
    pub hasTypeScriptDependency: bool,
    pub tsConfigOutDir: Option<String>,
    pub tsConfigRootDir: Option<String>,
    pub pythonPackageName: Option<String>,
}

pub struct MCPProjectAnalyzer {
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl MCPProjectAnalyzer {
    /// Creates an MCP project analyzer backed by the supplied file-system host.
    pub fn new(fileSystemHost: Arc<dyn FileSystemHost>) -> Self {
        Self { fileSystemHost }
    }

    #[allow(non_snake_case)]
    pub fn analyzeProjectStructure(
        &self,
        pluginDir: &Path,
        readmeContent: &str,
    ) -> ProjectStructure {
        let hasRequirementsTxt = self.fileExists(&pluginDir.join("requirements.txt"));
        let hasPyprojectToml = self.fileExists(&pluginDir.join("pyproject.toml"));
        let hasSetupPy = self.fileExists(&pluginDir.join("setup.py"));
        let hasPackageJson = self.fileExists(&pluginDir.join("package.json"));
        let hasTsConfig = self.fileExists(&pluginDir.join("tsconfig.json"));

        let pythonFiles = self.filesWithExtensions(pluginDir, &["py"]);
        let jsFiles = self.filesWithExtensions(pluginDir, &["js"]);
        let tsFiles = self.filesWithExtensions(pluginDir, &["ts", "tsx"]);
        let hasTsFiles = !tsFiles.is_empty();

        let mainPythonModule = self.findMainPythonModule(pluginDir);
        let packageJson = self.readJsonFile(&pluginDir.join("package.json"));
        let mainJsFile = self.findMainJsFile(pluginDir, packageJson.as_ref());
        let mainTsFile = self.findMainTsFile(pluginDir, packageJson.as_ref(), &tsFiles);
        let hasTypeScriptDependency = packageJson
            .as_ref()
            .map(hasTypeScriptMarker)
            .unwrap_or(false);
        let (tsConfigOutDir, tsConfigRootDir) = self.parseTsConfig(pluginDir);
        let pythonPackageName = self.parsePyprojectToml(pluginDir);

        let projectType = if hasTsConfig || hasTsFiles || hasTypeScriptDependency {
            ProjectType::TYPESCRIPT
        } else if hasPackageJson || !jsFiles.is_empty() {
            ProjectType::NODEJS
        } else if hasRequirementsTxt || hasPyprojectToml || hasSetupPy || !pythonFiles.is_empty() {
            ProjectType::PYTHON
        } else {
            ProjectType::UNKNOWN
        };

        let configExample =
            validateConfigExample(extractConfigExample(readmeContent), &projectType);

        ProjectStructure {
            r#type: projectType,
            hasRequirementsTxt,
            hasPyprojectToml,
            hasSetupPy,
            hasPackageJson,
            hasTsConfig,
            mainPythonModule,
            mainJsFile,
            mainTsFile,
            hasTsFiles,
            configExample,
            hasTypeScriptDependency,
            tsConfigOutDir,
            tsConfigRootDir,
            pythonPackageName,
        }
    }

    #[allow(non_snake_case)]
    pub fn findReadmeFile(&self, pluginDir: &Path) -> Option<PathBuf> {
        let candidates = [
            pluginDir.join("README.md"),
            pluginDir.join("readme.md"),
            pluginDir.join("INSTALL.md"),
            pluginDir.join("docs").join("README.md"),
            pluginDir.join("docs").join("readme.md"),
        ];
        for candidate in candidates {
            if self.fileExists(&candidate) {
                return Some(candidate);
            }
        }
        self.fileSystemHost
            .listFiles(pluginDir.to_str()?)
            .ok()?
            .into_iter()
            .find(|entry| {
                !entry.isDirectory
                    && Path::new(&entry.name)
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| value.eq_ignore_ascii_case("md"))
            })
            .map(|entry| pluginDir.join(entry.name))
    }

    /// Returns whether a host-owned path is an existing file.
    fn fileExists(&self, path: &Path) -> bool {
        path.to_str()
            .and_then(|path| self.fileSystemHost.fileExists(path).ok())
            .is_some_and(|entry| entry.exists && !entry.isDirectory)
    }

    /// Lists direct child files with extensions accepted by the project detector.
    fn filesWithExtensions(&self, dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
        let Some(path) = dir.to_str() else {
            return Vec::new();
        };
        self.fileSystemHost
            .listFiles(path)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                !entry.isDirectory
                    && Path::new(&entry.name)
                        .extension()
                        .and_then(|value| value.to_str())
                        .is_some_and(|value| {
                            extensions
                                .iter()
                                .any(|extension| value.eq_ignore_ascii_case(extension))
                        })
            })
            .map(|entry| dir.join(entry.name))
            .collect()
    }

    /// Reads and parses one JSON project file through the host file system.
    fn readJsonFile(&self, path: &Path) -> Option<Value> {
        let text = self.fileSystemHost.readFile(path.to_str()?).ok()?;
        serde_json::from_str::<Value>(&text).ok()
    }

    /// Finds the primary Python module declared by common project layouts.
    fn findMainPythonModule(&self, pluginDir: &Path) -> Option<String> {
        let srcDir = pluginDir.join("src");
        if srcDir
            .to_str()
            .and_then(|path| self.fileSystemHost.fileExists(path).ok())
            .is_some_and(|entry| entry.exists && entry.isDirectory)
        {
            for entry in self.fileSystemHost.listFiles(srcDir.to_str()?).ok()? {
                let path = srcDir.join(&entry.name);
                if entry.isDirectory && self.fileExists(&path.join("__init__.py")) {
                    return Some(entry.name);
                }
            }
        }
        for filename in ["main.py", "__main__.py", "app.py", "server.py"] {
            if self.fileExists(&pluginDir.join(filename)) {
                if filename == "__main__.py" {
                    return pluginDir
                        .file_name()
                        .and_then(|value| value.to_str())
                        .map(|value| value.replace('-', "_").to_ascii_lowercase());
                }
                return Some(filename.trim_end_matches(".py").to_string());
            }
        }
        if self.fileExists(&pluginDir.join("__init__.py")) {
            return pluginDir
                .file_name()
                .and_then(|value| value.to_str())
                .map(|value| value.replace('-', "_").to_ascii_lowercase());
        }
        let dirNameModule = pluginDir
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.replace('-', "_").to_ascii_lowercase())?;
        self.fileExists(&pluginDir.join(format!("{dirNameModule}.py")))
            .then_some(dirNameModule)
    }

    /// Finds the primary JavaScript file declared by package metadata or conventions.
    fn findMainJsFile(&self, pluginDir: &Path, packageJson: Option<&Value>) -> Option<String> {
        if let Some(main) = packageJson
            .and_then(|json| json.get("main"))
            .and_then(Value::as_str)
        {
            return Some(main.to_string());
        }
        ["index.js", "server.js", "app.js", "main.js"]
            .into_iter()
            .find(|filename| self.fileExists(&pluginDir.join(filename)))
            .map(str::to_string)
    }

    /// Finds the primary TypeScript file declared by metadata or source conventions.
    fn findMainTsFile(
        &self,
        pluginDir: &Path,
        packageJson: Option<&Value>,
        tsFiles: &[PathBuf],
    ) -> Option<String> {
        findMainTsFile(self, pluginDir, packageJson, tsFiles)
    }

    /// Reads TypeScript compiler directory settings from tsconfig.json.
    fn parseTsConfig(&self, pluginDir: &Path) -> (Option<String>, Option<String>) {
        let Some(json) = self.readJsonFile(&pluginDir.join("tsconfig.json")) else {
            return (None, None);
        };
        let options = json.get("compilerOptions");
        let outDir = options
            .and_then(|value| value.get("outDir"))
            .and_then(Value::as_str)
            .map(normalizePathText)
            .filter(|value| !value.is_empty());
        let rootDir = options
            .and_then(|value| value.get("rootDir"))
            .and_then(Value::as_str)
            .map(normalizePathText)
            .filter(|value| !value.is_empty());
        (outDir, rootDir)
    }

    /// Reads the Python package name from pyproject.toml through the host.
    fn parsePyprojectToml(&self, pluginDir: &Path) -> Option<String> {
        parsePyprojectTomlContent(
            &self
                .fileSystemHost
                .readFile(pluginDir.join("pyproject.toml").to_str()?)
                .ok()?,
        )
    }
}

#[allow(non_snake_case)]
fn hasTypeScriptMarker(packageJson: &Value) -> bool {
    let dependencyHasTypeScript = ["dependencies", "devDependencies"].iter().any(|key| {
        packageJson
            .get(key)
            .and_then(Value::as_object)
            .map(|items| items.contains_key("typescript") || items.contains_key("ts-node"))
            .unwrap_or(false)
    });
    if dependencyHasTypeScript {
        return true;
    }
    packageJson
        .get("scripts")
        .and_then(Value::as_object)
        .map(|scripts| {
            scripts.values().any(|value| {
                value
                    .as_str()
                    .map(|script| {
                        script.contains("tsc")
                            || script.contains("ts-node")
                            || script.contains("typescript")
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[allow(non_snake_case)]
fn findMainTsFile(
    analyzer: &MCPProjectAnalyzer,
    pluginDir: &Path,
    packageJson: Option<&Value>,
    tsFiles: &[PathBuf],
) -> Option<String> {
    if let Some(json) = packageJson {
        if let Some(bin) = json.get("bin") {
            let binPath = if let Some(value) = bin.as_str() {
                Some(value.to_string())
            } else {
                bin.as_object()
                    .and_then(|object| object.values().next())
                    .and_then(Value::as_str)
                    .map(str::to_string)
            };
            if let Some(binPath) = binPath {
                if binPath.ends_with(".ts") {
                    return Some(binPath);
                }
                if binPath.ends_with(".js") {
                    let jsFileName = binPath.rsplit('/').next().unwrap_or(&binPath);
                    let tsFileName = jsFileName.replace(".js", ".ts");
                    for candidate in [
                        format!("src/{tsFileName}"),
                        tsFileName,
                        binPath.replace(".js", ".ts"),
                    ] {
                        if analyzer.fileExists(&pluginDir.join(&candidate)) {
                            return Some(candidate);
                        }
                    }
                }
            }
        }
        if let Some(main) = json.get("main").and_then(Value::as_str) {
            if main.ends_with(".ts") || !main.contains('.') {
                return Some(main.to_string());
            }
        }
        if let Some(source) = json.get("source").and_then(Value::as_str) {
            if source.ends_with(".ts") {
                return Some(source.to_string());
            }
        }
    }
    for filename in [
        "src/index.ts",
        "src/main.ts",
        "src/app.ts",
        "src/server.ts",
        "index.ts",
        "server.ts",
        "app.ts",
        "main.ts",
    ] {
        if analyzer.fileExists(&pluginDir.join(filename)) {
            return Some(filename.to_string());
        }
    }
    tsFiles
        .first()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .map(str::to_string)
}

#[allow(non_snake_case)]
fn normalizePathText(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

#[allow(non_snake_case)]
fn parsePyprojectTomlContent(content: &str) -> Option<String> {
    let scriptsSection = Regex::new(r"(?s)\[project\.scripts\](.*?)(?:\n\[|$)").ok()?;
    if let Some(section) = scriptsSection
        .captures(&content)
        .and_then(|caps| caps.get(1))
    {
        let scriptPattern = Regex::new(r#"(?m)^\s*[\w-]+\s*=\s*"([^:"]+)"#).ok()?;
        if let Some(modulePath) = scriptPattern
            .captures(section.as_str())
            .and_then(|caps| caps.get(1))
            .map(|value| value.as_str().trim().to_string())
            .filter(|value| !value.is_empty() && !value.contains('/'))
        {
            return Some(modulePath);
        }
    }
    let packagesPattern = Regex::new(r#"packages\s*=\s*\["([^"]+)"\]"#).ok()?;
    if let Some(packageName) = packagesPattern
        .captures(&content)
        .and_then(|caps| caps.get(1))
        .and_then(|value| value.as_str().split('/').last())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
    {
        return Some(packageName);
    }
    let namePattern = Regex::new(r#"(?m)^\s*name\s*=\s*"([^"]+)"\s*$"#).ok()?;
    namePattern
        .captures(&content)
        .and_then(|caps| caps.get(1))
        .map(|value| value.as_str().replace('-', "_"))
        .filter(|value| !value.is_empty())
}

#[allow(non_snake_case)]
fn extractConfigExample(readmeContent: &str) -> Option<String> {
    let codeBlockRegex = Regex::new(r"(?s)```(?:bash|shell|cmd|sh|json)?(.*?)```").ok()?;
    for captures in codeBlockRegex.captures_iter(readmeContent) {
        let code = captures.get(1)?.as_str().trim();
        if code.contains("\"mcpServers\"")
            || code.contains("\"command\"")
            || code.contains("\"args\"")
        {
            return Some(code.to_string());
        }
    }
    let jsonConfigRegex = Regex::new(r#"(?s)\{.*?"mcpServers".*?\}"#).ok()?;
    jsonConfigRegex
        .find(readmeContent)
        .map(|value| value.as_str().to_string())
}

#[allow(non_snake_case)]
fn validateConfigExample(
    configExample: Option<String>,
    projectType: &ProjectType,
) -> Option<String> {
    let configExample = configExample?;
    let lowerCaseConfig = configExample.to_ascii_lowercase();
    if matches!(projectType, ProjectType::TYPESCRIPT | ProjectType::NODEJS)
        && !configExample.contains('@')
    {
        return None;
    }
    if matches!(projectType, ProjectType::PYTHON)
        && (lowerCaseConfig.contains("path/to") || lowerCaseConfig.contains("pathto/"))
    {
        return None;
    }
    Some(configExample)
}
