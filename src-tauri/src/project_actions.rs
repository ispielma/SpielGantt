use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project_payload::{open_project, OpenProjectError, OpenProjectResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectActionResult {
    project: OpenProjectResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReadmeEdit {
    pub readme_content: String,
    pub expected_readme_version: String,
}

impl ProjectActionResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }
}

pub fn edit_project_readme(
    project_path: &Path,
    edit: ProjectReadmeEdit,
) -> Result<ProjectActionResult, ProjectReadmeEditError> {
    let project = open_project(project_path).map_err(ProjectReadmeEditError::RefreshProject)?;
    let project_root = project
        .project_root()
        .ok_or_else(|| ProjectReadmeEditError::InvalidProject(project_path.to_path_buf()))?
        .to_path_buf();
    if !project.is_valid() {
        return Err(ProjectReadmeEditError::InvalidProject(project_root));
    }
    if project.project_readme_version() != edit.expected_readme_version {
        return Err(ProjectReadmeEditError::StaleReadme { project_root });
    }

    let readme_path = project_root.join("README.md");
    std::fs::write(&readme_path, edit.readme_content).map_err(|source| {
        ProjectReadmeEditError::WriteReadme {
            path: readme_path,
            source,
        }
    })?;

    Ok(ProjectActionResult {
        project: open_project(&project_root).map_err(ProjectReadmeEditError::RefreshProject)?,
    })
}

#[derive(Debug)]
pub enum ProjectReadmeEditError {
    RefreshProject(OpenProjectError),
    InvalidProject(PathBuf),
    StaleReadme {
        project_root: PathBuf,
    },
    WriteReadme {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ProjectReadmeEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RefreshProject(source) => write!(formatter, "{source}"),
            Self::InvalidProject(path) => {
                write!(
                    formatter,
                    "cannot edit project README for '{}': not a valid SpielGantt project",
                    path.display()
                )
            }
            Self::StaleReadme { project_root } => {
                write!(
                    formatter,
                    "README for project '{}' changed on disk; reload the project before saving",
                    project_root.display()
                )
            }
            Self::WriteReadme { path, source } => {
                write!(
                    formatter,
                    "failed to write project README '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ProjectReadmeEditError {}
