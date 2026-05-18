use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<WorkflowNodeConnection>,
    pub createdAt: i64,
    pub updatedAt: i64,
    pub enabled: bool,
    pub lastExecutionTime: Option<i64>,
    pub lastExecutionStatus: Option<ExecutionStatus>,
    pub totalExecutions: i32,
    pub successfulExecutions: i32,
    pub failedExecutions: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ExecutionStatus {
    SUCCESS,
    FAILED,
    RUNNING,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum WorkflowNode {
    #[serde(rename = "trigger")]
    Trigger(TriggerNode),
    #[serde(rename = "execute")]
    Execute(ExecuteNode),
    #[serde(rename = "condition")]
    Condition(ConditionNode),
    #[serde(rename = "logic")]
    Logic(LogicNode),
    #[serde(rename = "extract")]
    Extract(ExtractNode),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TriggerNode {
    pub id: String,
    #[serde(default = "default_trigger_node_type")]
    pub type_: String,
    pub name: String,
    pub description: String,
    pub position: NodePosition,
    pub triggerType: String,
    pub triggerConfig: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExecuteNode {
    pub id: String,
    #[serde(default = "default_execute_node_type")]
    pub type_: String,
    pub name: String,
    pub description: String,
    pub position: NodePosition,
    pub actionType: String,
    pub actionConfig: HashMap<String, ParameterValue>,
    pub jsCode: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ConditionOperator {
    EQ,
    NE,
    GT,
    GTE,
    LT,
    LTE,
    CONTAINS,
    NOT_CONTAINS,
    IN,
    NOT_IN,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ConditionNode {
    pub id: String,
    #[serde(default = "default_condition_node_type")]
    pub type_: String,
    pub name: String,
    pub description: String,
    pub position: NodePosition,
    pub left: ParameterValue,
    pub operator: ConditionOperator,
    pub right: ParameterValue,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum LogicOperator {
    AND,
    OR,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LogicNode {
    pub id: String,
    #[serde(default = "default_logic_node_type")]
    pub type_: String,
    pub name: String,
    pub description: String,
    pub position: NodePosition,
    pub operator: LogicOperator,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ExtractMode {
    REGEX,
    JSON,
    SUB,
    CONCAT,
    RANDOM_INT,
    RANDOM_STRING,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExtractNode {
    pub id: String,
    #[serde(default = "default_extract_node_type")]
    pub type_: String,
    pub name: String,
    pub description: String,
    pub position: NodePosition,
    pub source: ParameterValue,
    pub mode: ExtractMode,
    pub expression: String,
    pub group: i32,
    pub defaultValue: String,
    pub others: Vec<ParameterValue>,
    pub startIndex: i32,
    pub length: i32,
    pub randomMin: i32,
    pub randomMax: i32,
    pub randomStringLength: i32,
    pub randomStringCharset: String,
    pub useFixed: bool,
    pub fixedValue: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ParameterValue {
    StaticValue { value: String },
    NodeReference { nodeId: String },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkflowNodeConnection {
    pub id: String,
    pub sourceNodeId: String,
    pub targetNodeId: String,
    pub condition: Option<String>,
}

fn default_trigger_node_type() -> String {
    "trigger".to_string()
}

fn default_execute_node_type() -> String {
    "execute".to_string()
}

fn default_condition_node_type() -> String {
    "condition".to_string()
}

fn default_logic_node_type() -> String {
    "logic".to_string()
}

fn default_extract_node_type() -> String {
    "extract".to_string()
}
