use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Addresses one local layout instruction in a source instance, independently of host mounts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslReference {
    pub instanceId: String,
    pub node: u32,
}

/// Describes immutable layout work installed once instead of repeated for each row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslInstruction {
    #[serde(rename = "type")]
    pub nodeType: String,
    pub props: Value,
    pub children: Value,
    pub explicitChildren: bool,
}

/// Owns a module-versioned program whose operands reference constants, parameters, and local instructions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolPkgComposeDslProgram {
    pub id: String,
    pub root: u32,
    pub nodes: Vec<ToolPkgComposeDslInstruction>,
}

/// Carries an explicit public-node mutation without reconstructing the original layout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolPkgComposeDslPatch {
    pub node: u32,
    pub field: String,
    pub value: Value,
}

/// Carries only dynamic values and explicit edits for one source instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslInstance {
    pub id: String,
    pub programId: String,
    pub values: Vec<Value>,
    pub patches: Vec<ToolPkgComposeDslPatch>,
}

/// Applies one atomic revision of layout definitions and source instance data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgComposeDslDecodedTransaction {
    pub reset: bool,
    pub revision: u64,
    pub root: ToolPkgComposeDslReference,
    pub programs: Vec<ToolPkgComposeDslProgram>,
    pub instances: Vec<ToolPkgComposeDslInstance>,
    pub removed: Vec<String>,
}

/// Carries the exact arena allocated by the composition producer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgComposeDslTransaction {
    pub buffer: operit_link::CoreBuffer,
}

#[path = "ToolPkgComposeDslBuffer.rs"]
mod buffer;
pub use buffer::ComposeBuffer;
