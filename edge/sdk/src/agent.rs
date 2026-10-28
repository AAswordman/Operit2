//! Small bounded agent loop. The product owns model access and physical tools.
use std::{future::Future, pin::Pin};

pub type AgentFuture<'a> = Pin<Box<dyn Future<Output = Result<Step, AgentError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    InvalidLimits,
    InputTooLarge,
    OutputTooLarge,
    StepLimit,
    UnknownTool(String),
    DuplicateTool(&'static str),
    Backend(String),
    Tool(String),
}
impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AgentError {}

/// Limits bound retained context and tool execution. Backends must also bound
/// their own network response buffers and enforce request deadlines.
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_steps: usize,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_context_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_steps: 4,
            max_input_bytes: 2048,
            max_output_bytes: 2048,
            max_context_bytes: 4096,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON Schema for model providers that support structured tool calls.
    pub input_schema: &'static str,
}

/// Only registered product tools can execute. A tool validates its arguments
/// and enforces hardware limits before changing physical state.
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, arguments: &str) -> Result<String, AgentError>;
}

pub struct Observation {
    pub tool: String,
    pub arguments: String,
    pub result: String,
}

pub struct Request<'a> {
    pub instruction: &'a str,
    pub prompt: &'a str,
    pub tools: &'a [ToolSpec],
    pub observations: &'a [Observation],
}

pub enum Step {
    Reply(String),
    Call { tool: String, arguments: String },
}

/// Implement with a local model, an HTTP provider, or an authenticated Core
/// client. Model and transport choice do not change the device's tool surface.
pub trait Backend: Send + Sync {
    fn next<'a>(&'a self, request: Request<'a>) -> AgentFuture<'a>;
}

pub struct Agent<B> {
    backend: B,
    tools: Vec<Box<dyn Tool>>,
    specs: Vec<ToolSpec>,
    limits: Limits,
}
impl<B: Backend> Agent<B> {
    pub fn new(backend: B, limits: Limits) -> Result<Self, AgentError> {
        if limits.max_steps == 0
            || limits.max_input_bytes == 0
            || limits.max_output_bytes == 0
            || limits.max_context_bytes == 0
        {
            return Err(AgentError::InvalidLimits);
        }
        Ok(Self {
            backend,
            tools: Vec::new(),
            specs: Vec::new(),
            limits,
        })
    }

    pub fn register(&mut self, tool: impl Tool + 'static) -> Result<(), AgentError> {
        let spec = tool.spec();
        if self.specs.iter().any(|existing| existing.name == spec.name) {
            return Err(AgentError::DuplicateTool(spec.name));
        }
        self.specs.push(spec);
        self.tools.push(Box::new(tool));
        Ok(())
    }

    pub async fn run(&self, instruction: &str, prompt: &str) -> Result<String, AgentError> {
        if instruction.len().saturating_add(prompt.len()) > self.limits.max_input_bytes {
            return Err(AgentError::InputTooLarge);
        }
        let mut observations = Vec::new();
        let mut context_bytes = 0usize;
        for _ in 0..self.limits.max_steps {
            match self
                .backend
                .next(Request {
                    instruction,
                    prompt,
                    tools: &self.specs,
                    observations: &observations,
                })
                .await?
            {
                Step::Reply(text) => {
                    if text.len() > self.limits.max_output_bytes {
                        return Err(AgentError::OutputTooLarge);
                    }
                    return Ok(text);
                }
                Step::Call { tool, arguments } => {
                    if arguments.len() > self.limits.max_input_bytes {
                        return Err(AgentError::InputTooLarge);
                    }
                    let index = self
                        .specs
                        .iter()
                        .position(|spec| spec.name == tool)
                        .ok_or_else(|| AgentError::UnknownTool(tool.clone()))?;
                    // Reserve bounded context before performing a physical action.
                    let request_bytes = tool.len().saturating_add(arguments.len());
                    if context_bytes
                        .saturating_add(request_bytes)
                        .saturating_add(self.limits.max_output_bytes)
                        > self.limits.max_context_bytes
                    {
                        return Err(AgentError::OutputTooLarge);
                    }
                    let result = self.tools[index].call(&arguments)?;
                    if result.len() > self.limits.max_output_bytes {
                        return Err(AgentError::OutputTooLarge);
                    }
                    context_bytes += request_bytes + result.len();
                    observations.push(Observation {
                        tool,
                        arguments,
                        result,
                    });
                }
            }
        }
        Err(AgentError::StepLimit)
    }
}
