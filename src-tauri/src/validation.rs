use std::path::{Path, PathBuf};

use crate::diagnostics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    message: String,
}

impl ValidationIssue {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    project_root: Option<PathBuf>,
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new(project_root: Option<PathBuf>, issues: Vec<ValidationIssue>) -> Self {
        Self {
            project_root,
            issues,
        }
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn validate(start: &Path) -> Result<ValidationReport, ValidateError> {
    let diagnostics = diagnostics::read(start).map_err(ValidateError::ReadProjectDiagnostics)?;
    let issues = diagnostics
        .issues()
        .iter()
        .map(|issue| ValidationIssue::new(issue.message().to_string()))
        .collect();

    Ok(ValidationReport::new(
        diagnostics.project_root().map(Path::to_path_buf),
        issues,
    ))
}

#[derive(Debug)]
pub enum ValidateError {
    ReadProjectDiagnostics(diagnostics::ReadProjectDiagnosticsError),
}

impl std::fmt::Display for ValidateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadProjectDiagnostics(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for ValidateError {}
