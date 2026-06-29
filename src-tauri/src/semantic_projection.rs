use std::path::Path;

use crate::{
    dependency_relationships::{self, DependencyRelationships},
    event_axis_workflow::{self, EventAxisWorkflow},
    project_graph,
    project_snapshot::{self, ProjectSnapshot},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSemanticProjections {
    snapshot: ProjectSnapshot,
    relationships: DependencyRelationships,
    workflow: EventAxisWorkflow,
}

impl ProjectSemanticProjections {
    pub fn snapshot(&self) -> &ProjectSnapshot {
        &self.snapshot
    }

    pub fn relationships(&self) -> &DependencyRelationships {
        &self.relationships
    }

    pub fn workflow(&self) -> &EventAxisWorkflow {
        &self.workflow
    }
}

pub fn load(
    start: &Path,
) -> Result<ProjectSemanticProjections, LoadProjectSemanticProjectionsError> {
    let graph =
        project_graph::load(start).map_err(LoadProjectSemanticProjectionsError::LoadGraph)?;
    Ok(from_graph(&graph))
}

pub fn from_graph(graph: &project_graph::ProjectGraph) -> ProjectSemanticProjections {
    ProjectSemanticProjections {
        snapshot: project_snapshot::from_graph(graph),
        relationships: dependency_relationships::from_graph(graph),
        workflow: event_axis_workflow::from_graph(graph),
    }
}

#[derive(Debug)]
pub enum LoadProjectSemanticProjectionsError {
    LoadGraph(project_graph::LoadProjectGraphError),
}

impl std::fmt::Display for LoadProjectSemanticProjectionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadGraph(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for LoadProjectSemanticProjectionsError {}
