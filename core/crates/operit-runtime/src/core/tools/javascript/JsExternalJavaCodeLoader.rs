use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct JsExternalJavaCodeLoader {
    loadedArtifacts: Vec<LoadedArtifact>,
}

#[derive(Clone, Debug)]
struct LoadedArtifact {
    sourceType: String,
    sourcePath: String,
    nativeLibraryDir: Option<String>,
    childFirstPrefixes: Vec<String>,
}

impl JsExternalJavaCodeLoader {
    pub fn new() -> Self {
        Self {
            loadedArtifacts: Vec::new(),
        }
    }

    pub fn loadDex(&mut self, path: &str, optionsJson: &str) -> String {
        self.load("dex", path, optionsJson)
    }

    pub fn loadJar(&mut self, path: &str, optionsJson: &str) -> String {
        self.load("jar", path, optionsJson)
    }

    pub fn listLoadedArtifacts(&self) -> String {
        let payload = self
            .loadedArtifacts
            .iter()
            .enumerate()
            .map(|(index, artifact)| {
                json!({
                    "index": index,
                    "type": artifact.sourceType,
                    "path": artifact.sourcePath,
                    "nativeLibraryDir": artifact.nativeLibraryDir,
                    "childFirstPrefixes": artifact.childFirstPrefixes,
                    "alreadyLoaded": true
                })
            })
            .collect::<Vec<_>>();
        success(Value::Array(payload))
    }

    fn load(&mut self, sourceType: &str, path: &str, optionsJson: &str) -> String {
        let options = parseOptions(optionsJson);
        let artifact = LoadedArtifact {
            sourceType: sourceType.to_string(),
            sourcePath: path.trim().to_string(),
            nativeLibraryDir: options.nativeLibraryDir,
            childFirstPrefixes: options.childFirstPrefixes,
        };
        self.loadedArtifacts.push(artifact.clone());
        success(json!({
            "index": self.loadedArtifacts.len() - 1,
            "type": artifact.sourceType,
            "path": artifact.sourcePath,
            "nativeLibraryDir": artifact.nativeLibraryDir,
            "childFirstPrefixes": artifact.childFirstPrefixes,
            "alreadyLoaded": false
        }))
    }
}

impl Default for JsExternalJavaCodeLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
struct LoadOptions {
    nativeLibraryDir: Option<String>,
    childFirstPrefixes: Vec<String>,
}

fn parseOptions(optionsJson: &str) -> LoadOptions {
    let value = serde_json::from_str::<Value>(optionsJson).unwrap_or(Value::Null);
    let nativeLibraryDir = value
        .get("nativeLibraryDir")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let childFirstPrefixes = value
        .get("childFirstPrefixes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    LoadOptions {
        nativeLibraryDir,
        childFirstPrefixes,
    }
}

fn success(data: Value) -> String {
    json!({
        "success": true,
        "data": data
    })
    .to_string()
}
