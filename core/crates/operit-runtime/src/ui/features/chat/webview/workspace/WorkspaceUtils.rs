use std::fs;

use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_store::RuntimeStorageLayout::WORKSPACE_DIR_PATH;
use serde_json::{json, Value};

use crate::core::files::PathMapper::PathMapper;
use crate::ui::features::chat::webview::workspace::WorkspaceTemplateAssets::WORKSPACE_TEMPLATE_ASSETS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProjectType {
    WEB,
    ANDROID,
    FLUTTER,
    NODE,
    TYPESCRIPT,
    PYTHON,
    JAVA,
    GO,
    OFFICE,
    BLANK,
}

#[allow(non_snake_case)]
pub fn createAndGetDefaultWorkspace(
    chatId: String,
    projectType: Option<String>,
) -> Result<String, String> {
    let projectType = resolveProjectType(projectType)?;
    let workspaceRelativePath = getWorkspaceRelativePath(&chatId);

    if let Some(templateName) = projectType.templateName() {
        copyTemplateFiles(&workspaceRelativePath, templateName)?;
    }
    createProjectConfigIfNeeded(&workspaceRelativePath, projectType)?;

    PathMapper::workspacePath(&chatId)
}

#[allow(non_snake_case)]
pub fn createAndResetWorkspaceDirectory(chatId: String) -> Result<String, String> {
    let workspaceRelativePath = getWorkspaceRelativePath(&chatId);
    let storage = defaultRuntimeStorageHost();
    storage
        .delete(&workspaceRelativePath, true)
        .map_err(|error| error.to_string())?;
    ensureWorkspaceDirExists(&workspaceRelativePath)?;
    PathMapper::workspacePath(&chatId)
}

#[allow(non_snake_case)]
fn getWorkspaceRelativePath(chatId: &str) -> String {
    format!("{WORKSPACE_DIR_PATH}/{chatId}")
}

#[allow(non_snake_case)]
fn ensureWorkspaceDirExists(workspaceRelativePath: &str) -> Result<(), String> {
    let storage = defaultRuntimeStorageHost();
    let root = storage
        .rootDir()
        .expect("RuntimeStorageHost root must be configured for WorkspaceUtils");
    fs::create_dir_all(root.join(workspaceRelativePath)).map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn resolveProjectType(projectType: Option<String>) -> Result<ProjectType, String> {
    let Some(projectType) = projectType else {
        return Ok(ProjectType::WEB);
    };
    match projectType.trim() {
        "web" => Ok(ProjectType::WEB),
        "android" => Ok(ProjectType::ANDROID),
        "flutter" => Ok(ProjectType::FLUTTER),
        "node" => Ok(ProjectType::NODE),
        "typescript" => Ok(ProjectType::TYPESCRIPT),
        "python" => Ok(ProjectType::PYTHON),
        "java" => Ok(ProjectType::JAVA),
        "go" => Ok(ProjectType::GO),
        "office" => Ok(ProjectType::OFFICE),
        "blank" => Ok(ProjectType::BLANK),
        value => Err(format!("unknown workspace project type: {value}")),
    }
}

impl ProjectType {
    #[allow(non_snake_case)]
    fn templateName(self) -> Option<&'static str> {
        match self {
            ProjectType::WEB => Some("web"),
            ProjectType::ANDROID => Some("android"),
            ProjectType::FLUTTER => Some("flutter"),
            ProjectType::NODE => Some("node"),
            ProjectType::TYPESCRIPT => Some("typescript"),
            ProjectType::PYTHON => Some("python"),
            ProjectType::JAVA => Some("java"),
            ProjectType::GO => Some("go"),
            ProjectType::OFFICE => Some("office"),
            ProjectType::BLANK => None,
        }
    }

    #[allow(non_snake_case)]
    fn config(self) -> Value {
        match self {
            ProjectType::WEB => generateWebProjectConfig(),
            ProjectType::ANDROID => generateAndroidProjectConfig(),
            ProjectType::FLUTTER => generateFlutterProjectConfig(),
            ProjectType::NODE => generateNodeProjectConfig(),
            ProjectType::TYPESCRIPT => generateTypeScriptProjectConfig(),
            ProjectType::PYTHON => generatePythonProjectConfig(),
            ProjectType::JAVA => generateJavaProjectConfig(),
            ProjectType::GO => generateGoProjectConfig(),
            ProjectType::OFFICE => generateOfficeProjectConfig(),
            ProjectType::BLANK => generateBlankProjectConfig(),
        }
    }
}

#[allow(non_snake_case)]
fn copyTemplateFiles(workspaceRelativePath: &str, templateName: &str) -> Result<(), String> {
    let prefix = format!("{templateName}/");
    let mut matched = false;
    let storage = defaultRuntimeStorageHost();

    for asset in WORKSPACE_TEMPLATE_ASSETS {
        let Some(relativePath) = asset.path.strip_prefix(&prefix) else {
            continue;
        };
        matched = true;
        let destinationRelativePath = templateDestinationPath(relativePath);
        storage
            .writeBytes(
                &format!("{workspaceRelativePath}/{destinationRelativePath}"),
                asset.bytes,
            )
            .map_err(|error| error.to_string())?;
    }

    if !matched {
        return Err(format!(
            "workspace template asset not found: {templateName}"
        ));
    }
    Ok(())
}

#[allow(non_snake_case)]
fn templateDestinationPath(relativePath: &str) -> String {
    relativePath
        .split('/')
        .map(|segment| {
            if segment == "gitignore" {
                ".gitignore"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[allow(non_snake_case)]
fn createProjectConfigIfNeeded(
    workspaceRelativePath: &str,
    projectType: ProjectType,
) -> Result<(), String> {
    let storage = defaultRuntimeStorageHost();
    let configPath = format!("{workspaceRelativePath}/.operit/config.json");
    if storage
        .exists(&configPath)
        .map_err(|error| error.to_string())?
    {
        return Ok(());
    }
    let content =
        serde_json::to_string_pretty(&projectType.config()).map_err(|error| error.to_string())?;
    storage
        .writeBytes(&configPath, content.as_bytes())
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
fn generateBlankProjectConfig() -> Value {
    json!({
        "projectType": "blank",
        "title": "Blank Workspace",
        "description": "This is a blank workspace with only basic directory structure. You can edit .operit/config.json to configure project type, server, and commands.",
        "server": {"enabled": false, "port": 8080, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false, "previewButtonLabel": ""},
        "commands": [],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateAndroidProjectConfig() -> Value {
    json!({
        "projectType": "android",
        "title": "Android Project",
        "description": "Android application workspace.",
        "server": {"enabled": false, "port": 8080, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false, "previewButtonLabel": ""},
        "commands": [
            command("android_setup_env", "Setup Android Environment", "bash setup_android_env.sh"),
            command("gradle_assemble_debug", "Assemble Debug", "./gradlew assembleDebug"),
            command("gradle_assemble_release", "Assemble Release", "./gradlew assembleRelease"),
            toolCommand("android_install_debug_apk", "Install Debug APK", "install_app", json!({"path": "$WORKSPACE/app/build/outputs/apk/debug/app-debug.apk"})),
            toolCommand("share_release_apk", "Share Release APK", "share_file", json!({"path": "$WORKSPACE/app/build/outputs/apk/release/app-release.apk", "title": "Share Release APK"})),
            command("gradle_lint", "Lint", "./gradlew lint"),
            command("gradle_test", "Test", "./gradlew test")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateFlutterProjectConfig() -> Value {
    json!({
        "projectType": "flutter",
        "title": "Flutter Project",
        "description": "Flutter application workspace.",
        "server": {"enabled": false, "port": 5013, "autoStart": false},
        "preview": {"type": "terminal", "url": "http://localhost:5013", "showPreviewButton": true, "previewButtonLabel": "Browser Preview"},
        "commands": [
            command("flutter_android_setup_env", "Setup Android Environment", "bash android/setup_android_env.sh"),
            command("flutter_doctor", "Flutter Doctor", "flutter doctor"),
            command("flutter_pub_get", "Flutter Pub Get", "flutter pub get"),
            dedicatedCommand("flutter_run_web_server", "Run Web Server", "flutter run -d web-server --web-hostname 0.0.0.0 --web-port 5013", "Flutter Web Server"),
            command("flutter_analyze", "Flutter Analyze", "flutter analyze"),
            command("flutter_test", "Flutter Test", "flutter test"),
            command("flutter_build_apk", "Build APK", "flutter build apk"),
            command("flutter_build_web", "Build Web", "flutter build web --no-tree-shake-icons")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateWebProjectConfig() -> Value {
    json!({
        "projectType": "web",
        "title": "Web Project",
        "description": "Static web workspace.",
        "server": {"enabled": true, "port": 8093, "autoStart": true},
        "preview": {"type": "browser", "url": "http://localhost:8093"},
        "commands": [],
        "export": {"enabled": true}
    })
}

#[allow(non_snake_case)]
fn generateNodeProjectConfig() -> Value {
    json!({
        "projectType": "node",
        "title": "Node.js Project",
        "description": "Node.js workspace.",
        "server": {"enabled": false, "port": 3000, "autoStart": false},
        "preview": {"type": "terminal", "url": "http://localhost:3000", "showPreviewButton": true, "previewButtonLabel": "Browser Preview"},
        "commands": [
            command("npm_init", "npm init -y", "npm init -y"),
            command("npm_install", "npm install", "npm install"),
            dedicatedCommand("npm_start", "npm start", "npm start", "npm start"),
            command("npm_test", "npm test", "npm test")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateTypeScriptProjectConfig() -> Value {
    json!({
        "projectType": "typescript",
        "title": "TypeScript Project",
        "description": "TypeScript workspace.",
        "server": {"enabled": false, "port": 3000, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false},
        "commands": [
            command("pnpm_install", "pnpm install", "pnpm install"),
            command("pnpm_build", "pnpm build", "pnpm build"),
            dedicatedCommand("tsc_watch", "tsc watch", "pnpm exec tsc --watch", "TypeScript Watch"),
            dedicatedCommand("pnpm_start", "pnpm start", "pnpm start", "pnpm start"),
            command("pnpm_list", "pnpm list", "pnpm list")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generatePythonProjectConfig() -> Value {
    json!({
        "projectType": "python",
        "title": "Python Project",
        "description": "Python workspace.",
        "server": {"enabled": false, "port": 8000, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false},
        "commands": [
            command("venv_create", "Create venv", "python -m venv venv"),
            command("venv_activate", "Activate venv", "source venv/bin/activate || venv\\Scripts\\activate"),
            command("pip_install", "pip install", "pip install -r requirements.txt"),
            command("pip_list", "pip list", "pip list"),
            command("python_run", "Run Python", "python main.py")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateJavaProjectConfig() -> Value {
    json!({
        "projectType": "java",
        "title": "Java Project",
        "description": "Java workspace.",
        "server": {"enabled": false, "port": 8080, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false},
        "commands": [
            command("gradle_init", "Gradle Init", "gradle wrapper --gradle-version 8.5"),
            command("gradle_build", "Gradle Build", "./gradlew build || gradle build"),
            command("gradle_run", "Gradle Run", "./gradlew run || gradle run"),
            command("gradle_test", "Gradle Test", "./gradlew test || gradle test"),
            command("gradle_jar", "Gradle Jar", "./gradlew jar || gradle jar"),
            command("gradle_clean", "Gradle Clean", "./gradlew clean || gradle clean"),
            command("gradle_tasks", "Gradle Tasks", "./gradlew tasks || gradle tasks")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateGoProjectConfig() -> Value {
    json!({
        "projectType": "go",
        "title": "Go Project",
        "description": "Go workspace.",
        "server": {"enabled": false, "port": 8080, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false},
        "commands": [
            command("go_mod_init", "go mod init", "go mod init myapp"),
            command("go_mod_tidy", "go mod tidy", "go mod tidy"),
            command("go_run", "go run main.go", "go run main.go"),
            command("go_build", "go build", "go build")
        ],
        "export": {"enabled": false}
    })
}

#[allow(non_snake_case)]
fn generateOfficeProjectConfig() -> Value {
    json!({
        "projectType": "office",
        "title": "Office Workspace",
        "description": "Office document workspace.",
        "server": {"enabled": false, "port": 8080, "autoStart": false},
        "preview": {"type": "terminal", "url": "", "showPreviewButton": false, "previewButtonLabel": ""},
        "commands": [],
        "export": {"enabled": false}
    })
}

fn command(id: &str, label: &str, command: &str) -> Value {
    json!({
        "id": id,
        "label": label,
        "command": command,
        "workingDir": ".",
        "shell": true
    })
}

#[allow(non_snake_case)]
fn dedicatedCommand(id: &str, label: &str, command: &str, sessionTitle: &str) -> Value {
    json!({
        "id": id,
        "label": label,
        "command": command,
        "workingDir": ".",
        "shell": true,
        "usesDedicatedSession": true,
        "sessionTitle": sessionTitle
    })
}

#[allow(non_snake_case)]
fn toolCommand(id: &str, label: &str, tool: &str, toolParameters: Value) -> Value {
    json!({
        "id": id,
        "label": label,
        "tool": tool,
        "toolParameters": toolParameters,
        "workingDir": ".",
        "shell": true
    })
}
