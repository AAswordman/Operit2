pub enum WorkflowLogLevel {
    INFO,
    WARN,
    ERROR,
}

pub struct WorkflowExecutionLogEntry {
    pub level: WorkflowLogLevel,
    pub message: String,
}

pub struct WorkflowExecutionRecord {
    pub id: String,
    pub entries: Vec<WorkflowExecutionLogEntry>,
}
