pub enum ToolPkgSourceType {
    ASSET,
    EXTERNAL,
}

pub struct ToolPkgResourceRuntime {
    pub key: String,
    pub path: String,
    pub mime: String,
}

pub struct ToolPkgParser;
