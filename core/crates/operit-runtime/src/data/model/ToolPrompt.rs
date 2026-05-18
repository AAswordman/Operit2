use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolParameterSchema {
    pub name: String,
    pub r#type: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

impl ToolParameterSchema {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            r#type: "string".to_string(),
            description,
            required: true,
            default: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolPrompt {
    pub name: String,
    pub description: String,
    pub parameters: String,
    pub parametersStructured: Option<Vec<ToolParameterSchema>>,
    pub details: String,
    pub notes: String,
}

impl ToolPrompt {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            parameters: String::new(),
            parametersStructured: None,
            details: String::new(),
            notes: String::new(),
        }
    }
}

impl fmt::Display for ToolPrompt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "- {}: {}", self.name, self.description)?;

        let paramsString = match &self.parametersStructured {
            Some(parametersStructured) if !parametersStructured.is_empty() => {
                parametersStructured
                    .iter()
                    .map(|param| {
                        let fullDesc = match &param.default {
                            Some(default) if !param.description.contains("default") => {
                                format!("{}, default {}", param.description, default)
                            }
                            _ => param.description.clone(),
                        };
                        format!("{} ({})", param.name, fullDesc)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
            _ => self.parameters.clone(),
        };

        if !paramsString.is_empty() {
            write!(formatter, " Parameters: {}", paramsString)?;
        }

        if !self.details.is_empty() {
            write!(formatter, "\n{}", self.details)?;
        }

        if !self.notes.is_empty() {
            write!(formatter, "\n{}", self.notes)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemToolPromptCategory {
    pub categoryName: String,
    pub categoryHeader: String,
    pub tools: Vec<ToolPrompt>,
    pub categoryFooter: String,
}

impl fmt::Display for SystemToolPromptCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if !self.categoryName.is_empty() {
            parts.push(format!("{}:", self.categoryName));
        }
        if !self.categoryHeader.is_empty() {
            parts.push(self.categoryHeader.clone());
        }
        if !self.tools.is_empty() {
            parts.push(
                self.tools
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        if !self.categoryFooter.is_empty() {
            parts.push(self.categoryFooter.clone());
        }
        write!(formatter, "{}", parts.join("\n"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageToolPromptCategory {
    pub packageName: String,
    pub packageDescription: String,
    pub tools: Vec<ToolPrompt>,
}

impl fmt::Display for PackageToolPromptCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Package: {}\nDescription: {}",
            self.packageName, self.packageDescription
        )?;
        if !self.tools.is_empty() {
            write!(
                formatter,
                "\nTools:\n{}",
                self.tools
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            )?;
        }
        Ok(())
    }
}
