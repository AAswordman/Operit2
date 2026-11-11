use operit_host_api::HostManager::HostManager;

pub fn parse_bool_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("invalid bool: {other}; expected true | false")),
    }
}

pub fn parse_i32_arg(value: Option<&String>, usage: &str) -> Result<i32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i32>()
        .map_err(|error| error.to_string())
}

pub fn parse_f32_arg(value: Option<&String>, usage: &str) -> Result<f32, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<f32>()
        .map_err(|error| error.to_string())
}

pub fn parse_i64_arg(value: Option<&String>, usage: &str) -> Result<i64, String> {
    value
        .ok_or_else(|| usage.to_string())?
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

#[allow(non_snake_case)]
pub fn parseCsvList(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if !trimmed.is_empty() && !result.iter().any(|entry| entry == trimmed) {
            result.push(trimmed.to_string());
        }
    }
    result
}

pub fn parse_on_off_arg(value: Option<&String>, usage: &str) -> Result<bool, String> {
    match value.ok_or_else(|| usage.to_string())?.as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("invalid switch: {other}; expected on | off")),
    }
}

/// Resolves command file input against the invocation's explicit virtual directory.
pub fn command_file_path(context: &HostManager, path: &str) -> Result<String, String> {
    let storage = context
        .runtimeStorageHost
        .clone()
        .ok_or("Command file access requires runtime storage")?;
    let directory =
        operit_tools::ToolExecutionManager::ToolExecutionManager::currentToolRuntimeContext()
            .map(|turn| turn.fileDirectory(storage))
            .transpose()?
            .flatten();
    context
        .vfs()?
        .resolveResourcePath(directory.as_deref(), path)
}
/// Reads literal content or a file through the owning runtime VFS.
pub fn read_content_arg(context: &HostManager, value: &str) -> Result<String, String> {
    if let Some(path) = value.strip_prefix('@') {
        let filesystem = context.vfs()?;
        let path = command_file_path(context, path)?;
        return filesystem.readFile(&path);
    }
    Ok(value.to_string())
}
