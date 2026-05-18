pub struct ToolPkgComposeDslNode {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<ToolPkgComposeDslNode>,
    pub text: Option<String>,
}

pub struct ToolPkgComposeDslRenderResult {
    pub root: Option<ToolPkgComposeDslNode>,
}

pub struct ToolPkgComposeDslParser;
