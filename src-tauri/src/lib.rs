pub mod agent_scaffold;
mod app_mutation_refresh;
pub mod dependency_relationships;
pub mod event_axis_workflow;
pub mod package_mutation;
pub mod project_graph;
mod project_lifecycle;
mod project_namespace;
mod project_payload;
mod relative_task_insert;
pub mod runtime;
pub mod semantic_projection;
mod task_actions;
pub mod task_adoptable_folders;
mod task_edit_action;
mod task_metadata_mutations;
mod task_package_index;
pub mod metadata {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    pub struct ProjectMetadata {
        pub schema_version: u32,
        pub folder_naming: FolderNamingPolicy,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub events: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub boundary_events: Option<ProjectBoundaryEvents>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    pub struct ProjectBoundaryEvents {
        pub start: String,
        pub finish: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum FolderNamingPolicy {
        TaskId,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ProjectNamespaceKind {
        Task,
        Event,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    pub struct TaskMetadata {
        pub schema_version: u32,
        pub id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ends_at: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub dependencies: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub status: Option<TaskStatus>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(rename_all = "snake_case")]
    pub enum TaskStatus {
        #[serde(alias = "planned", alias = "in_progress")]
        Unblocked,
        Blocked,
        Done,
    }

    impl ProjectMetadata {
        pub const DEFAULT_EVENTS: [&'static str; 2] = ["start", "finished"];

        pub fn from_json(input: &str) -> Result<Self, MetadataError> {
            let metadata: Self = serde_json::from_str(input)?;
            if metadata.schema_version != 1 {
                return Err(MetadataError::UnsupportedSchemaVersion(
                    metadata.schema_version,
                ));
            }
            validate_project_events(metadata.events.as_deref())?;
            validate_project_boundary_events(
                metadata.events.as_deref(),
                metadata.boundary_events.as_ref(),
            )?;
            Ok(metadata)
        }

        pub fn version_1() -> Self {
            let events = Self::DEFAULT_EVENTS
                .iter()
                .map(|event| (*event).to_string())
                .collect::<Vec<_>>();
            Self {
                schema_version: 1,
                folder_naming: FolderNamingPolicy::TaskId,
                boundary_events: Some(ProjectBoundaryEvents {
                    start: events[0].clone(),
                    finish: events[1].clone(),
                }),
                events: Some(events),
            }
        }

        pub fn to_json(&self) -> Result<String, MetadataError> {
            validate_project_events(self.events.as_deref())?;
            validate_project_boundary_events(
                self.events.as_deref(),
                self.boundary_events.as_ref(),
            )?;
            serde_json::to_string_pretty(self)
                .map(|contents| format!("{contents}\n"))
                .map_err(MetadataError::from)
        }

        pub fn resolved_boundary_events(&self) -> Option<ProjectBoundaryEvents> {
            self.boundary_events
                .clone()
                .or_else(|| infer_boundary_events(self.events.as_deref()))
        }

        pub fn chart_events(&self) -> Vec<String> {
            chart_events(
                self.events.clone().unwrap_or_default(),
                self.resolved_boundary_events(),
            )
        }
    }

    impl TaskMetadata {
        pub fn from_json(input: &str) -> Result<Self, MetadataError> {
            let metadata: Self = serde_json::from_str(input)?;
            if metadata.schema_version != 1 {
                return Err(MetadataError::UnsupportedSchemaVersion(
                    metadata.schema_version,
                ));
            }
            validate_task_id(&metadata.id)?;
            validate_task_ends_at(metadata.ends_at.as_deref())?;
            Ok(metadata)
        }

        pub fn version_1(id: impl Into<String>) -> Result<Self, MetadataError> {
            let id = id.into();
            validate_task_id(&id)?;
            Ok(Self {
                schema_version: 1,
                id,
                ends_at: None,
                dependencies: Vec::new(),
                status: None,
            })
        }

        pub fn to_json(&self) -> Result<String, MetadataError> {
            validate_task_id(&self.id)?;
            validate_task_ends_at(self.ends_at.as_deref())?;
            serde_json::to_string_pretty(self)
                .map(|contents| format!("{contents}\n"))
                .map_err(MetadataError::from)
        }
    }

    pub fn validate_project_namespace_entry(
        id: &str,
        kind: ProjectNamespaceKind,
        existing_kind: ProjectNamespaceKind,
        existing_ids: &[String],
    ) -> Result<(), MetadataError> {
        validate_single_path_component(id, namespace_kind_label(kind)).map_err(match kind {
            ProjectNamespaceKind::Task => MetadataError::InvalidTaskId,
            ProjectNamespaceKind::Event => MetadataError::InvalidProjectEventId,
        })?;

        if existing_ids.iter().any(|existing_id| existing_id == id) {
            return Err(MetadataError::ProjectNamespaceCollision {
                id: id.to_string(),
                kind,
                existing_kind,
            });
        }

        Ok(())
    }

    pub(crate) fn validate_single_path_component(value: &str, subject: &str) -> Result<(), String> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(format!("{subject} must not be empty"));
        }

        if trimmed != value {
            return Err(format!("{subject} must not start or end with whitespace"));
        }

        if matches!(trimmed, "." | "..") {
            return Err(format!("{subject} must not be '.' or '..'"));
        }

        if trimmed.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
                )
        }) {
            return Err(format!(
                "{subject} must not contain filesystem separator or reserved filename characters"
            ));
        }

        let reserved_names = [
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
        ];
        if reserved_names.contains(&trimmed.to_ascii_lowercase().as_str()) {
            return Err(format!(
                "{subject} '{trimmed}' is reserved by common filesystems"
            ));
        }

        Ok(())
    }

    pub fn validate_task_id(id: &str) -> Result<(), MetadataError> {
        validate_single_path_component(id, "task id").map_err(MetadataError::InvalidTaskId)
    }

    fn namespace_kind_label(kind: ProjectNamespaceKind) -> &'static str {
        match kind {
            ProjectNamespaceKind::Task => "task id",
            ProjectNamespaceKind::Event => "event id",
        }
    }

    fn validate_project_events(events: Option<&[String]>) -> Result<(), MetadataError> {
        let Some(events) = events else {
            return Ok(());
        };

        let mut seen = std::collections::HashSet::new();
        for event in events {
            validate_single_path_component(event, "event id")
                .map_err(MetadataError::InvalidProjectEventId)?;
            if !seen.insert(event) {
                return Err(MetadataError::DuplicateProjectEventId(event.clone()));
            }
        }

        Ok(())
    }

    fn validate_project_boundary_events(
        events: Option<&[String]>,
        boundary_events: Option<&ProjectBoundaryEvents>,
    ) -> Result<(), MetadataError> {
        let Some(boundary_events) = boundary_events else {
            return Ok(());
        };

        validate_single_path_component(&boundary_events.start, "start boundary event id")
            .map_err(MetadataError::InvalidProjectBoundaryEvents)?;
        validate_single_path_component(&boundary_events.finish, "finish boundary event id")
            .map_err(MetadataError::InvalidProjectBoundaryEvents)?;
        if boundary_events.start == boundary_events.finish {
            return Err(MetadataError::InvalidProjectBoundaryEvents(
                "start and finish boundary events must be different".to_string(),
            ));
        }

        let Some(events) = events else {
            return Err(MetadataError::InvalidProjectBoundaryEvents(
                "boundary events must reference project events".to_string(),
            ));
        };
        for event_id in [&boundary_events.start, &boundary_events.finish] {
            if !events.iter().any(|event| event == event_id) {
                return Err(MetadataError::InvalidProjectBoundaryEvents(format!(
                    "boundary event id '{event_id}' is not in the project event list"
                )));
            }
        }

        Ok(())
    }

    fn infer_boundary_events(events: Option<&[String]>) -> Option<ProjectBoundaryEvents> {
        let events = events?;
        if events.len() < 2 {
            return None;
        }

        Some(ProjectBoundaryEvents {
            start: events.first()?.clone(),
            finish: events.last()?.clone(),
        })
    }

    fn chart_events(
        events: Vec<String>,
        boundary_events: Option<ProjectBoundaryEvents>,
    ) -> Vec<String> {
        let Some(boundary_events) = boundary_events else {
            return events;
        };

        let mut ordered = Vec::with_capacity(events.len());
        if events.iter().any(|event| event == &boundary_events.start) {
            ordered.push(boundary_events.start.clone());
        }
        ordered.extend(
            events
                .iter()
                .filter(|event| {
                    *event != &boundary_events.start && *event != &boundary_events.finish
                })
                .cloned(),
        );
        if events.iter().any(|event| event == &boundary_events.finish) {
            ordered.push(boundary_events.finish);
        }
        ordered
    }

    fn validate_task_ends_at(ends_at: Option<&str>) -> Result<(), MetadataError> {
        let Some(ends_at) = ends_at else {
            return Ok(());
        };

        validate_single_path_component(ends_at, "event id")
            .map_err(MetadataError::InvalidProjectEventId)
    }

    #[derive(Debug)]
    pub enum MetadataError {
        InvalidTaskId(String),
        InvalidProjectEventId(String),
        InvalidProjectBoundaryEvents(String),
        DuplicateProjectEventId(String),
        ProjectNamespaceCollision {
            id: String,
            kind: ProjectNamespaceKind,
            existing_kind: ProjectNamespaceKind,
        },
        InvalidTaskMetadata(String),
        UnsupportedSchemaVersion(u32),
        Json(serde_json::Error),
    }

    impl std::fmt::Display for MetadataError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::InvalidTaskId(message) => write!(formatter, "{message}"),
                Self::InvalidProjectEventId(message) => write!(formatter, "{message}"),
                Self::InvalidProjectBoundaryEvents(message) => write!(formatter, "{message}"),
                Self::DuplicateProjectEventId(event_id) => {
                    write!(formatter, "duplicate event id '{event_id}'")
                }
                Self::ProjectNamespaceCollision {
                    id,
                    kind,
                    existing_kind,
                } => write!(
                    formatter,
                    "{} '{}' collides with project {} '{}'",
                    namespace_kind_label(*kind),
                    id,
                    namespace_kind_label(*existing_kind),
                    id
                ),
                Self::InvalidTaskMetadata(message) => write!(formatter, "{message}"),
                Self::UnsupportedSchemaVersion(version) => {
                    write!(formatter, "unsupported metadata schema version {version}")
                }
                Self::Json(error) => write!(formatter, "{error}"),
            }
        }
    }

    impl std::error::Error for MetadataError {}

    impl From<serde_json::Error> for MetadataError {
        fn from(error: serde_json::Error) -> Self {
            Self::Json(error)
        }
    }
}

fn path_points_to_distinct_existing_entry(
    path: &std::path::Path,
    current_path: &std::path::Path,
) -> bool {
    if !path.exists() {
        return false;
    }

    match (
        std::fs::canonicalize(path),
        std::fs::canonicalize(current_path),
    ) {
        (Ok(path), Ok(current_path)) => path != current_path,
        _ => true,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataRewrite {
    path: std::path::PathBuf,
    original_contents: String,
    updated_contents: String,
}

impl MetadataRewrite {
    pub fn new(
        path: impl Into<std::path::PathBuf>,
        original_contents: impl Into<String>,
        updated_contents: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            original_contents: original_contents.into(),
            updated_contents: updated_contents.into(),
        }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn original_contents(&self) -> &str {
        &self.original_contents
    }

    pub fn updated_contents(&self) -> &str {
        &self.updated_contents
    }
}

fn plan_loaded_task_metadata_rewrites<E, F, S>(
    loaded_tasks: &[project_graph::LoadedProjectGraphTask],
    mut edit_task_metadata: F,
    mut serialize_error: S,
) -> Result<Vec<MetadataRewrite>, E>
where
    F: FnMut(
        &project_graph::LoadedProjectGraphTask,
        &mut metadata::TaskMetadata,
    ) -> Result<Option<std::path::PathBuf>, E>,
    S: FnMut(metadata::MetadataError) -> E,
{
    let mut rewrites = Vec::new();

    for loaded_task in loaded_tasks {
        let mut task_metadata = loaded_task.metadata.clone();
        let rewrite_path = edit_task_metadata(loaded_task, &mut task_metadata)?
            .unwrap_or_else(|| loaded_task.path.join(".spielgantt").join("task.json"));
        let updated_contents = task_metadata.to_json().map_err(&mut serialize_error)?;
        rewrites.push(MetadataRewrite {
            path: rewrite_path,
            original_contents: loaded_task.original_metadata_contents.clone(),
            updated_contents,
        });
    }

    Ok(rewrites)
}

fn apply_metadata_rewrites_with_rollback(
    rewrites: &[MetadataRewrite],
) -> Result<(), package_mutation::AtomicJsonMetadataWriteError> {
    package_mutation::PackageMutation::metadata_rewrites(rewrites.iter().cloned())
        .commit()
        .map(|_| ())
        .map_err(package_mutation::MetadataCommitError::into_source)
}

pub mod project {
    use std::path::{Path, PathBuf};

    use crate::metadata::{MetadataError, ProjectMetadata};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum InitProjectOutcome {
        Created,
        AlreadyExists,
    }

    pub fn init(target: &Path) -> Result<InitProjectOutcome, InitProjectError> {
        let target_metadata =
            std::fs::metadata(target).map_err(|source| InitProjectError::InvalidTargetPath {
                path: target.to_path_buf(),
                source,
            })?;
        if !target_metadata.is_dir() {
            return Err(InitProjectError::TargetIsNotDirectory(target.to_path_buf()));
        }

        let spielgantt_dir = target.join(".spielgantt");
        std::fs::create_dir_all(&spielgantt_dir).map_err(|source| InitProjectError::CreateDir {
            path: spielgantt_dir.clone(),
            source,
        })?;

        let project_metadata_path = spielgantt_dir.join("project.json");
        if project_metadata_path.exists() {
            return Ok(InitProjectOutcome::AlreadyExists);
        }

        let project_metadata = ProjectMetadata::version_1()
            .to_json()
            .map_err(InitProjectError::SerializeMetadata)?;
        std::fs::write(&project_metadata_path, project_metadata).map_err(|source| {
            InitProjectError::WriteMetadata {
                path: project_metadata_path,
                source,
            }
        })?;

        Ok(InitProjectOutcome::Created)
    }

    pub fn create_in_parent(
        project_name: &str,
        projects_root: &Path,
    ) -> Result<PathBuf, CreateProjectError> {
        crate::metadata::validate_single_path_component(project_name, "project name")
            .map_err(CreateProjectError::InvalidProjectName)?;

        let project_root = projects_root.join(project_name);

        if std::fs::metadata(&project_root).is_ok() {
            return Err(CreateProjectError::ProjectAlreadyExists(project_root));
        }

        std::fs::create_dir_all(&projects_root).map_err(|source| {
            CreateProjectError::CreateProjectsDir {
                path: projects_root.to_path_buf(),
                source,
            }
        })?;
        std::fs::create_dir(&project_root).map_err(|source| {
            CreateProjectError::CreateProjectDir {
                path: project_root.clone(),
                source,
            }
        })?;
        init(&project_root).map_err(CreateProjectError::InitProject)?;

        Ok(project_root)
    }

    pub fn find_root(start: &Path) -> Option<PathBuf> {
        let start = std::fs::canonicalize(start).ok()?;
        start.ancestors().find_map(|candidate| {
            candidate
                .join(".spielgantt")
                .join("project.json")
                .is_file()
                .then(|| candidate.to_path_buf())
        })
    }

    pub fn read_metadata(project_root: &Path) -> Result<ProjectMetadata, ReadProjectMetadataError> {
        let project_metadata_path = project_root.join(".spielgantt").join("project.json");
        let project_metadata_contents =
            std::fs::read_to_string(&project_metadata_path).map_err(|source| {
                ReadProjectMetadataError::ReadMetadata {
                    path: project_metadata_path.clone(),
                    source,
                }
            })?;
        ProjectMetadata::from_json(&project_metadata_contents).map_err(|source| {
            ReadProjectMetadataError::ParseMetadata {
                path: project_metadata_path,
                source,
            }
        })
    }

    #[derive(Debug)]
    pub enum InitProjectError {
        InvalidTargetPath {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
        TargetIsNotDirectory(std::path::PathBuf),
        CreateDir {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
        SerializeMetadata(crate::metadata::MetadataError),
        WriteMetadata {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
    }

    impl std::fmt::Display for InitProjectError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::InvalidTargetPath { path, source } => {
                    write!(
                        formatter,
                        "invalid project path '{}': {source}",
                        path.display()
                    )
                }
                Self::TargetIsNotDirectory(path) => {
                    write!(
                        formatter,
                        "invalid project path '{}': target is not a directory",
                        path.display()
                    )
                }
                Self::CreateDir { path, source } => {
                    write!(
                        formatter,
                        "failed to create metadata directory '{}': {source}",
                        path.display()
                    )
                }
                Self::SerializeMetadata(source) => {
                    write!(formatter, "failed to serialize project metadata: {source}")
                }
                Self::WriteMetadata { path, source } => {
                    write!(
                        formatter,
                        "failed to write project metadata '{}': {source}",
                        path.display()
                    )
                }
            }
        }
    }

    impl std::error::Error for InitProjectError {}

    #[derive(Debug)]
    pub enum CreateProjectError {
        InvalidProjectName(String),
        ProjectAlreadyExists(PathBuf),
        CreateProjectsDir {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
        CreateProjectDir {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
        InitProject(InitProjectError),
    }

    impl std::fmt::Display for CreateProjectError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::InvalidProjectName(message) => write!(formatter, "{message}"),
                Self::ProjectAlreadyExists(path) => write!(
                    formatter,
                    "project directory '{}' already exists",
                    path.display()
                ),
                Self::CreateProjectsDir { path, source } => {
                    write!(
                        formatter,
                        "failed to create projects directory '{}': {source}",
                        path.display()
                    )
                }
                Self::CreateProjectDir { path, source } => {
                    write!(
                        formatter,
                        "failed to create project directory '{}': {source}",
                        path.display()
                    )
                }
                Self::InitProject(source) => write!(formatter, "{source}"),
            }
        }
    }

    impl std::error::Error for CreateProjectError {}

    #[derive(Debug)]
    pub enum ReadProjectMetadataError {
        ReadMetadata {
            path: std::path::PathBuf,
            source: std::io::Error,
        },
        ParseMetadata {
            path: std::path::PathBuf,
            source: MetadataError,
        },
    }

    impl std::fmt::Display for ReadProjectMetadataError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::ReadMetadata { path, source } => {
                    write!(
                        formatter,
                        "failed to read project metadata '{}': {source}",
                        path.display()
                    )
                }
                Self::ParseMetadata { path, source } => {
                    write!(
                        formatter,
                        "failed to parse project metadata '{}': {source}",
                        path.display()
                    )
                }
            }
        }
    }

    impl std::error::Error for ReadProjectMetadataError {}
}

pub mod snapshot;
pub use snapshot as project_snapshot;

pub(crate) mod diagnostics;

pub mod export {
    use std::path::Path;

    use crate::{metadata::TaskStatus, project_snapshot};

    pub fn project_markdown(start: &Path) -> Result<String, ExportProjectError> {
        let snapshot =
            project_snapshot::load(start).map_err(ExportProjectError::LoadProjectSnapshot)?;

        let mut markdown = String::from("# SpielGantt Project Snapshot\n\n");
        if !snapshot.events().is_empty() {
            markdown.push_str("## Events\n\n");
            for (index, event) in snapshot.events().iter().enumerate() {
                markdown.push_str(&format!("{}. {event}\n", index + 1));
            }
            markdown.push('\n');
        }

        for (index, task) in snapshot.tasks().iter().enumerate() {
            markdown.push_str(&format!("## Task `{}`\n\n", task.id()));
            markdown.push_str(&format!(
                "- Path: `{}`\n",
                task.project_relative_path().display()
            ));
            markdown.push_str(&format!("- Status: {}\n", format_status(task.status())));
            if let Some(ends_at) = task.ends_at() {
                markdown.push_str(&format!("- Ends at: `{ends_at}`\n"));
            }

            let dependencies = task.dependency_references();
            if dependencies.is_empty() {
                markdown.push_str("- Dependencies: none\n");
            } else {
                markdown.push_str("- Dependencies:\n");
                for dependency in dependencies {
                    markdown.push_str(&format!(
                        "  - `{}` ({})\n",
                        dependency.id(),
                        match dependency.kind() {
                            project_snapshot::ProjectSnapshotDependencyKind::Task => "task",
                            project_snapshot::ProjectSnapshotDependencyKind::Event => "event",
                        }
                    ));
                }
            }

            if index + 1 < snapshot.tasks().len() {
                markdown.push('\n');
            }
        }

        Ok(markdown)
    }

    fn format_status(status: Option<&TaskStatus>) -> String {
        status
            .map(|status| match status {
                TaskStatus::Unblocked => "`unblocked`".to_string(),
                TaskStatus::Blocked => "`blocked`".to_string(),
                TaskStatus::Done => "`done`".to_string(),
            })
            .unwrap_or_else(|| "-".to_string())
    }

    #[derive(Debug)]
    pub enum ExportProjectError {
        LoadProjectSnapshot(project_snapshot::LoadProjectSnapshotError),
    }

    impl std::fmt::Display for ExportProjectError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::LoadProjectSnapshot(source) => write!(formatter, "{source}"),
            }
        }
    }

    impl std::error::Error for ExportProjectError {}
}

pub mod cache {
    use std::path::{Path, PathBuf};

    use serde::{Deserialize, Serialize};

    use crate::project_snapshot;

    const TASK_CACHE_SCHEMA_VERSION: u32 = 1;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RebuildCacheOutcome {
        path: PathBuf,
        task_count: usize,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CachedTask {
        id: String,
        path: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    struct TaskCacheMetadata {
        schema_version: u32,
        tasks: Vec<TaskCacheEntry>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    struct TaskCacheEntry {
        id: String,
        path: String,
    }

    impl RebuildCacheOutcome {
        pub fn path(&self) -> &Path {
            &self.path
        }

        pub fn task_count(&self) -> usize {
            self.task_count
        }
    }

    impl CachedTask {
        pub fn id(&self) -> &str {
            &self.id
        }

        pub fn path(&self) -> &str {
            &self.path
        }
    }

    pub fn rebuild(start: &Path) -> Result<RebuildCacheOutcome, RebuildCacheError> {
        let snapshot =
            project_snapshot::load(start).map_err(RebuildCacheError::LoadProjectSnapshot)?;
        let tasks = snapshot
            .tasks()
            .iter()
            .map(|task| TaskCacheEntry {
                id: task.id().to_string(),
                path: task.project_relative_path().display().to_string(),
            })
            .collect::<Vec<_>>();

        let task_count = tasks.len();
        let cache_metadata = TaskCacheMetadata {
            schema_version: TASK_CACHE_SCHEMA_VERSION,
            tasks,
        };
        let cache_contents = serde_json::to_string_pretty(&cache_metadata)
            .map(|contents| format!("{contents}\n"))
            .map_err(RebuildCacheError::SerializeCache)?;
        let cache_dir = snapshot.project_root().join(".spielgantt").join("cache");
        std::fs::create_dir_all(&cache_dir).map_err(|source| {
            RebuildCacheError::CreateCacheDir {
                path: cache_dir.clone(),
                source,
            }
        })?;
        let cache_path = cache_dir.join("tasks.json");
        std::fs::write(&cache_path, cache_contents).map_err(|source| {
            RebuildCacheError::WriteCache {
                path: cache_path.clone(),
                source,
            }
        })?;

        Ok(RebuildCacheOutcome {
            path: cache_path,
            task_count,
        })
    }

    pub(crate) fn read_for_project_root(
        project_root: &Path,
    ) -> Result<Option<Vec<CachedTask>>, ReadCacheError> {
        let cache_path = project_root
            .join(".spielgantt")
            .join("cache")
            .join("tasks.json");
        if !cache_path.is_file() {
            return Ok(None);
        }

        let cache_contents =
            std::fs::read_to_string(&cache_path).map_err(|source| ReadCacheError::ReadCache {
                path: cache_path.clone(),
                source,
            })?;
        let cache_metadata: TaskCacheMetadata =
            serde_json::from_str(&cache_contents).map_err(|source| ReadCacheError::ParseCache {
                path: cache_path.clone(),
                source,
            })?;
        if cache_metadata.schema_version != TASK_CACHE_SCHEMA_VERSION {
            return Err(ReadCacheError::UnsupportedSchemaVersion {
                path: cache_path,
                version: cache_metadata.schema_version,
            });
        }

        Ok(Some(
            cache_metadata
                .tasks
                .into_iter()
                .map(|task| CachedTask {
                    id: task.id,
                    path: task.path,
                })
                .collect(),
        ))
    }

    #[derive(Debug)]
    pub enum RebuildCacheError {
        LoadProjectSnapshot(project_snapshot::LoadProjectSnapshotError),
        CreateCacheDir {
            path: PathBuf,
            source: std::io::Error,
        },
        SerializeCache(serde_json::Error),
        WriteCache {
            path: PathBuf,
            source: std::io::Error,
        },
    }

    impl std::fmt::Display for RebuildCacheError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::LoadProjectSnapshot(source) => write!(formatter, "{source}"),
                Self::CreateCacheDir { path, source } => {
                    write!(
                        formatter,
                        "failed to create cache directory '{}': {source}",
                        path.display()
                    )
                }
                Self::SerializeCache(source) => {
                    write!(formatter, "failed to serialize task cache: {source}")
                }
                Self::WriteCache { path, source } => {
                    write!(
                        formatter,
                        "failed to write task cache '{}': {source}",
                        path.display()
                    )
                }
            }
        }
    }

    impl std::error::Error for RebuildCacheError {}

    #[derive(Debug)]
    pub(crate) enum ReadCacheError {
        ReadCache {
            path: PathBuf,
            source: std::io::Error,
        },
        ParseCache {
            path: PathBuf,
            source: serde_json::Error,
        },
        UnsupportedSchemaVersion {
            path: PathBuf,
            version: u32,
        },
    }

    impl std::fmt::Display for ReadCacheError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::ReadCache { path, source } => {
                    write!(
                        formatter,
                        "failed to read task cache '{}': {source}",
                        path.display()
                    )
                }
                Self::ParseCache { path, source } => {
                    write!(
                        formatter,
                        "failed to parse task cache '{}': {source}",
                        path.display()
                    )
                }
                Self::UnsupportedSchemaVersion { path, version } => {
                    write!(
                        formatter,
                        "unsupported task cache schema version {version} in '{}'",
                        path.display()
                    )
                }
            }
        }
    }

    impl std::error::Error for ReadCacheError {}
}

pub mod repair {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    use crate::{cache, diagnostics};

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RepairIssue {
        message: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RepairReport {
        project_root: PathBuf,
        issues: Vec<RepairIssue>,
    }

    impl RepairIssue {
        pub fn message(&self) -> &str {
            &self.message
        }
    }

    impl RepairReport {
        pub fn project_root(&self) -> &Path {
            &self.project_root
        }

        pub fn issues(&self) -> &[RepairIssue] {
            &self.issues
        }

        pub fn is_clean(&self) -> bool {
            self.issues.is_empty()
        }
    }

    pub fn report(start: &Path) -> Result<RepairReport, RepairError> {
        let diagnostics = diagnostics::read(start).map_err(RepairError::ReadProjectDiagnostics)?;
        let project_root = diagnostics
            .project_root()
            .ok_or_else(|| RepairError::ProjectNotFound(start.to_path_buf()))?
            .to_path_buf();
        let mut issues = Vec::new();

        match cache::read_for_project_root(&project_root) {
            Ok(Some(cached_tasks)) => {
                issues.extend(compare_cached_tasks(&cached_tasks, diagnostics.tasks()));
            }
            Ok(None) => {}
            Err(error) => {
                issues.push(RepairIssue {
                    message: format!("{error}; run `spielgantt cache rebuild` to recreate it"),
                });
            }
        }

        for issue in diagnostics.issues() {
            issues.push(RepairIssue {
                message: issue.message().to_string(),
            });
        }

        Ok(RepairReport {
            project_root,
            issues,
        })
    }

    fn compare_cached_tasks(
        cached_tasks: &[cache::CachedTask],
        current_tasks: &[diagnostics::ProjectDiagnosticTask],
    ) -> Vec<RepairIssue> {
        let current_paths_by_id = current_tasks
            .iter()
            .map(|task| {
                (
                    task.id().to_string(),
                    task.project_relative_path().to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut issues = Vec::new();

        for cached_task in cached_tasks {
            match current_paths_by_id.get(cached_task.id()) {
                Some(current_path) if current_path.as_str() != cached_task.path() => {
                    issues.push(RepairIssue {
                        message: format!(
                            "task '{}' appears moved or renamed: {} -> {}",
                            cached_task.id(),
                            cached_task.path(),
                            current_path
                        ),
                    });
                }
                Some(_) => {}
                None => {
                    issues.push(RepairIssue {
                        message: format!(
                            "cached task '{}' is missing from {}",
                            cached_task.id(),
                            cached_task.path()
                        ),
                    });
                }
            }
        }

        issues
    }

    #[derive(Debug)]
    pub enum RepairError {
        ProjectNotFound(PathBuf),
        ReadProjectDiagnostics(diagnostics::ReadProjectDiagnosticsError),
    }

    impl std::fmt::Display for RepairError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::ProjectNotFound(path) => {
                    write!(
                        formatter,
                        "cannot repair from '{}': current directory is not inside a SpielGantt project",
                        path.display()
                    )
                }
                Self::ReadProjectDiagnostics(source) => write!(formatter, "{source}"),
            }
        }
    }

    impl std::error::Error for RepairError {}
}

pub mod mutation_plan;

pub mod task;

pub mod event;

pub mod validation;

pub mod platform_open {
    use std::path::Path;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Platform {
        Macos,
        Windows,
        Linux,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OpenCommand {
        program: String,
        args: Vec<String>,
    }

    impl OpenCommand {
        pub fn program(&self) -> &str {
            &self.program
        }

        pub fn args(&self) -> &[String] {
            &self.args
        }
    }

    pub fn command_for(platform: Platform, target: &Path) -> OpenCommand {
        let target = target.display().to_string();
        match platform {
            Platform::Macos => OpenCommand {
                program: "open".to_string(),
                args: vec![target],
            },
            Platform::Windows => OpenCommand {
                program: "explorer".to_string(),
                args: vec![target],
            },
            Platform::Linux => OpenCommand {
                program: "xdg-open".to_string(),
                args: vec![target],
            },
        }
    }

    pub fn command_for_current_platform(target: &Path) -> Result<OpenCommand, OpenPathError> {
        let platform = current_platform().ok_or(OpenPathError::UnsupportedPlatform)?;
        Ok(command_for(platform, target))
    }

    pub fn open_path(target: &Path) -> Result<OpenCommand, OpenPathError> {
        let open_command = command_for_current_platform(target)?;
        let status = std::process::Command::new(open_command.program())
            .args(open_command.args())
            .status()
            .map_err(|source| OpenPathError::Spawn {
                command: open_command.clone(),
                source,
            })?;

        if !status.success() {
            return Err(OpenPathError::Failed {
                command: open_command,
                status,
            });
        }

        Ok(open_command)
    }

    fn current_platform() -> Option<Platform> {
        if cfg!(target_os = "macos") {
            Some(Platform::Macos)
        } else if cfg!(target_os = "windows") {
            Some(Platform::Windows)
        } else if cfg!(target_os = "linux") {
            Some(Platform::Linux)
        } else {
            None
        }
    }

    #[derive(Debug)]
    pub enum OpenPathError {
        UnsupportedPlatform,
        Spawn {
            command: OpenCommand,
            source: std::io::Error,
        },
        Failed {
            command: OpenCommand,
            status: std::process::ExitStatus,
        },
    }

    impl std::fmt::Display for OpenPathError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::UnsupportedPlatform => {
                    write!(formatter, "opening paths is not supported on this platform")
                }
                Self::Spawn { command, source } => {
                    write!(
                        formatter,
                        "failed to run platform open command '{} {}': {source}",
                        command.program(),
                        command.args().join(" ")
                    )
                }
                Self::Failed { command, status } => {
                    write!(
                        formatter,
                        "platform open command '{} {}' exited with {status}",
                        command.program(),
                        command.args().join(" ")
                    )
                }
            }
        }
    }

    impl std::error::Error for OpenPathError {}
}

pub mod app_facade;
pub mod project_actions;
mod tauri_commands;
pub use tauri_commands::run;
