use std::{fs, path::Path};

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

fn filesystem_is_case_insensitive(root: &Path) -> bool {
    let probe_dir = root.join("CaseProbe");
    fs::create_dir(&probe_dir).expect("probe directory should be created");
    let is_case_insensitive = root.join("caseprobe").exists();
    fs::remove_dir(&probe_dir).expect("probe directory should be removed");
    is_case_insensitive
}

fn direct_child_names(root: &Path) -> Vec<String> {
    let mut names = fs::read_dir(root)
        .expect("directory entries should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should load")
                .file_name()
                .into_string()
                .expect("directory entry name should be valid utf-8")
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn read_task_metadata(path: impl AsRef<Path>) -> spielgantt_lib::metadata::TaskMetadata {
    spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(path.as_ref()).expect("task metadata should be readable"),
    )
    .expect("task metadata should remain valid JSON")
}

#[test]
fn app_task_dependency_add_persists_and_refreshes_project_relationships() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("blocker task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");

    let result = spielgantt_lib::app_facade::add_task_dependency(
        &project_dir,
        "analyze-results",
        "calibrate-laser",
    )
    .expect("application dependency add should succeed");

    let refreshed_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("refreshed application project should include the dependent task");
    assert_eq!(
        refreshed_task.dependencies(),
        &["calibrate-laser".to_string()]
    );
    let metadata = read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        metadata
            .dependencies
            .contains(&"calibrate-laser".to_string()),
        "application dependency add should persist through the shared core dependency writer"
    );
}

#[test]
fn app_task_dependency_add_reports_refresh_error_after_successful_mutation() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("blocker task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    std::fs::write(project_dir.join("analyze-results/README.md"), [0xff, 0xfe])
        .expect("invalid task README bytes should be written");

    let error = spielgantt_lib::app_facade::add_task_dependency(
        &project_dir,
        "analyze-results",
        "calibrate-laser",
    )
    .expect_err("successful dependency mutation should surface the failed project refresh");

    assert!(
        error.to_string().contains("failed to read task README"),
        "refresh failure should be reported after a successful mutation: {error}"
    );
    let metadata = read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        metadata
            .dependencies
            .contains(&"calibrate-laser".to_string()),
        "dependency mutation should persist before the refresh failure is reported"
    );
}

#[test]
fn app_task_create_failure_preserves_mutation_error_without_refreshing_broken_project() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("existing task creation should succeed");
    std::fs::write(project_dir.join("calibrate-laser/README.md"), [0xff, 0xfe])
        .expect("invalid task README bytes should be written");

    let error = spielgantt_lib::app_facade::create_task(&project_dir, "calibrate-laser")
        .expect_err("duplicate task mutation should fail");
    let message = error.to_string();

    assert!(
        message.contains("task id 'calibrate-laser' already exists"),
        "mutation failure should preserve the package mutation error: {message}"
    );
    assert!(
        !message.contains("failed to read task README"),
        "failed mutations must not perform a misleading project refresh: {message}"
    );
}

#[test]
fn app_task_insert_before_uses_shared_core_operation_and_refreshes_project() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "prepare-inputs")
        .expect("blocker task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analysis-run")
        .expect("selected task creation should succeed");
    std::fs::write(
        project_dir.join("analysis-run/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"prepare-inputs\"\n  ]\n}\n",
    )
    .expect("selected task metadata should be rewritten with a blocker");

    let result = spielgantt_lib::app_facade::insert_task_before(
        &project_dir,
        "analysis-run",
        "quality-gate",
    )
    .expect("application insert-before should succeed");

    let inserted_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "quality-gate")
        .expect("refreshed application project should include the inserted task");
    assert_eq!(
        inserted_task.dependencies(),
        &["prepare-inputs".to_string()],
        "inserted task should inherit selected task blockers through the shared core operation"
    );
    let selected_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analysis-run")
        .expect("refreshed application project should include the selected task");
    assert_eq!(
        selected_task.dependencies(),
        &["quality-gate".to_string()],
        "selected task should depend on the inserted predecessor in refreshed application state"
    );
}

#[test]
fn app_task_insert_after_uses_shared_core_operation_and_refreshes_project() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "analysis-run")
        .expect("selected task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "write-report")
        .expect("downstream task creation should succeed");
    std::fs::write(
        project_dir.join("write-report/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"write-report\",\n  \"dependencies\": [\n    \"analysis-run\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be rewritten with the selected blocker");

    let result = spielgantt_lib::app_facade::insert_task_after(
        &project_dir,
        "analysis-run",
        "archive-results",
    )
    .expect("application insert-after should succeed");

    let inserted_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "archive-results")
        .expect("refreshed application project should include the inserted task");
    assert_eq!(
        inserted_task.dependencies(),
        &["analysis-run".to_string()],
        "inserted task should depend on the selected predecessor"
    );
    let downstream_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "write-report")
        .expect("refreshed application project should include the downstream task");
    assert_eq!(
        downstream_task.dependencies(),
        &["archive-results".to_string()],
        "downstream task should be rewired through the shared core operation"
    );
}

#[test]
fn app_task_ends_at_refreshes_the_project_snapshot_and_heals_dependencies() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("ending task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("downstream task creation should succeed");
    std::fs::write(
        project_dir.join("analyze-results/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be written");

    let healed = spielgantt_lib::app_facade::set_task_ends_at(
        &project_dir,
        "prepare-samples",
        Some("START"),
        false,
    )
    .expect("application task ends_at should succeed");

    let ending_task = healed
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "prepare-samples")
        .expect("refreshed application project should include the ending task");
    assert_eq!(
        ending_task.ends_at(),
        Some("START"),
        "ending task should keep the selected event anchor"
    );
    let downstream_task = healed
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("refreshed application project should include the downstream task");
    assert_eq!(
        downstream_task.dependencies(),
        &["START".to_string()],
        "application task ends_at should heal downstream dependencies through the shared core path"
    );
    let downstream_metadata =
        read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        downstream_metadata
            .dependencies
            .contains(&"START".to_string()),
        "healed dependency should be persisted through the shared core path"
    );

    let cleared =
        spielgantt_lib::app_facade::set_task_ends_at(&project_dir, "prepare-samples", None, true)
            .expect("application task ends_at clear should succeed");

    let cleared_ending_task = cleared
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "prepare-samples")
        .expect("refreshed application project should include the cleared ending task");
    assert_eq!(
        cleared_ending_task.ends_at(),
        None,
        "clearing ends_at should remove the event anchor"
    );
    let cleared_downstream_task = cleared
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("refreshed application project should include the downstream task after clear");
    assert_eq!(
        cleared_downstream_task.dependencies(),
        &["START".to_string()],
        "clearing ends_at should preserve existing event dependencies"
    );
}

#[test]
fn app_task_ends_at_rejects_reversed_event_spans_without_rewriting_metadata() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"start\",\n    \"samples ready\",\n    \"data ready\",\n    \"analysis complete\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    spielgantt_lib::task::create(&project_dir, "analysis-run")
        .expect("analysis task creation should succeed");
    let task_metadata_path = project_dir.join("analysis-run/.spielgantt/task.json");
    std::fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"data ready\"\n  ]\n}\n",
    )
    .expect("task metadata should be written with an event blocker");
    let original_metadata =
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let error = spielgantt_lib::app_facade::set_task_ends_at(
        &project_dir,
        "analysis-run",
        Some("samples ready"),
        false,
    )
    .expect_err("application task ends_at should reject a reversed event span");

    assert!(
        error.to_string().contains(
            "cannot set task 'analysis-run' to end at event 'samples ready' before existing event blocker 'data ready'"
        ),
        "application/shared rejection should identify the requested end event and blocking event: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after application/shared rejection"),
        original_metadata,
        "application/shared rejection should not rewrite task metadata"
    );
}

#[test]
fn app_task_mutation_refreshes_the_same_workflow_domain_contract_as_cli_json() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("ending task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("downstream task creation should succeed");

    let result = spielgantt_lib::app_facade::set_task_ends_at(
        &project_dir,
        "prepare-samples",
        Some("BEC"),
        false,
    )
    .expect("application task ends_at should succeed");

    let cli_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        cli_output.status.success(),
        "CLI workflow query should succeed after application mutation: {cli_output:?}"
    );
    let cli_workflow: serde_json::Value =
        serde_json::from_str(&stdout(&cli_output)).expect("CLI workflow should be JSON");
    let result_json =
        serde_json::to_value(&result).expect("application task result should serialize");

    assert_eq!(
        result_json["project"]["workflow"], cli_workflow,
        "Tauri/shared mutation result should refresh the same workflow domain contract as CLI JSON"
    );
}

#[test]
fn app_task_rename_persists_name_folder_dependencies_and_refreshes_project_snapshot() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written");
    std::fs::write(
        project_dir.join("calibrate-laser").join("notes.txt"),
        "keep the experiment notes",
    )
    .expect("task user file should be written");

    let result =
        spielgantt_lib::app_facade::rename_task(&project_dir, "calibrate-laser", "align-optics")
            .expect("application task rename should succeed");

    let refreshed_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "align-optics")
        .expect("refreshed application project should include the renamed task");
    assert!(
        refreshed_task.dependencies().is_empty(),
        "renamed task should not invent dependencies"
    );
    assert!(
        project_dir.join("align-optics").is_dir(),
        "application task rename should rename the task folder to match the new name"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "application task rename should remove the old task folder"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("align-optics/notes.txt"))
            .expect("user file should remain readable after rename"),
        "keep the experiment notes",
        "application task rename should preserve user files inside the renamed folder"
    );
    let renamed_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(project_dir.join("align-optics/.spielgantt/task.json"))
            .expect("renamed task metadata should be readable"),
    )
    .expect("renamed task metadata should remain valid JSON");
    assert_eq!(
        renamed_metadata.id, "align-optics",
        "application task rename should update the renamed task metadata id"
    );
    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(project_dir.join("analyze-results/.spielgantt/task.json"))
            .expect("dependent task metadata should be readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert!(
        dependent_metadata
            .dependencies
            .contains(&"align-optics".to_string()),
        "application task rename should update dependency references to the new task name"
    );
    assert!(
        result.project().tasks().iter().any(|task| {
            task.id() == "analyze-results" && task.dependencies() == &["align-optics".to_string()]
        }),
        "application task rename should refresh the project snapshot after updating dependencies"
    );
}

#[test]
fn app_task_delete_remove_from_chart_removes_metadata_keeps_user_files_and_heals_dependencies() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "collect-samples")
        .expect("deleted task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    std::fs::write(
        project_dir.join("collect-samples").join("notes.txt"),
        "keep user notes",
    )
    .expect("task user file should be written");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"collect-samples\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written");

    let result = spielgantt_lib::app_facade::delete_task(
        &project_dir,
        "collect-samples",
        spielgantt_lib::task::DeleteTaskMode::RemoveFromChart,
    )
    .expect("application task delete should succeed");

    assert!(
        !project_dir
            .join("collect-samples")
            .join(".spielgantt")
            .exists(),
        "remove from chart should remove SpielGantt metadata"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("collect-samples").join("notes.txt"))
            .expect("task user file should remain readable"),
        "keep user notes",
        "remove from chart should preserve user-owned files"
    );
    let dependent_metadata =
        read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        dependent_metadata.dependencies.is_empty(),
        "deleting a task should remove stale dependency references"
    );
    assert!(
        !result
            .project()
            .tasks()
            .iter()
            .any(|task| task.id() == "collect-samples"),
        "refreshed project should no longer include the removed task"
    );
    assert!(
        result
            .project()
            .tasks()
            .iter()
            .any(|task| { task.id() == "analyze-results" && task.dependencies().is_empty() }),
        "refreshed project should report healed dependent task metadata"
    );
}

#[test]
fn app_task_delete_directory_removes_task_bucket_and_heals_dependencies() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "collect-samples")
        .expect("deleted task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    std::fs::write(
        project_dir.join("collect-samples").join("notes.txt"),
        "discard user notes",
    )
    .expect("task user file should be written");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"collect-samples\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written");

    let result = spielgantt_lib::app_facade::delete_task(
        &project_dir,
        "collect-samples",
        spielgantt_lib::task::DeleteTaskMode::DeleteDirectory,
    )
    .expect("application task directory delete should succeed");

    assert!(
        !project_dir.join("collect-samples").exists(),
        "delete directory should remove the whole task bucket"
    );
    let dependent_metadata =
        read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        dependent_metadata.dependencies.is_empty(),
        "directory deletion should remove stale dependency references"
    );
    assert!(
        !result
            .project()
            .tasks()
            .iter()
            .any(|task| task.id() == "collect-samples"),
        "refreshed project should no longer include the deleted task"
    );
}

#[test]
fn app_task_rename_rejects_invalid_or_colliding_new_names_before_writing_changes() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "align-optics")
        .expect("colliding task creation should succeed");

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let invalid_result =
        spielgantt_lib::app_facade::rename_task(&project_dir, "calibrate-laser", "Align/Optics");
    let invalid_error =
        invalid_result.expect_err("application task rename should reject invalid names");
    assert!(
        invalid_error.to_string().contains("filesystem separator"),
        "invalid rename should explain the filesystem-safe name rule"
    );
    assert_eq!(
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "invalid rename should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "invalid rename should leave the original task folder in place"
    );

    let duplicate_result =
        spielgantt_lib::app_facade::rename_task(&project_dir, "calibrate-laser", "align-optics");
    let duplicate_error =
        duplicate_result.expect_err("application task rename should reject collisions");
    assert!(
        duplicate_error.to_string().contains("already exists"),
        "collision rejection should explain the target folder conflict"
    );
    assert_eq!(
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "collision rejection should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "collision rejection should leave the original task folder in place"
    );
}

#[test]
fn app_task_rename_rejects_new_names_that_collide_with_project_events() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let collision_result =
        spielgantt_lib::app_facade::rename_task(&project_dir, "calibrate-laser", "START");
    let collision_error =
        collision_result.expect_err("application task rename should reject event id collisions");
    assert!(
        collision_error
            .to_string()
            .contains("collides with project event id 'START'"),
        "event-collision rejection should mention the shared namespace rule"
    );
    assert_eq!(
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "event-collision rejection should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "event-collision rejection should leave the original task folder in place"
    );
}

#[test]
fn app_task_rename_allows_case_only_folder_alignment_on_case_insensitive_filesystems() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    if !filesystem_is_case_insensitive(workspace.path()) {
        return;
    }

    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "Calibrate Laser")
        .expect("task creation should succeed");
    std::fs::write(
        project_dir.join("Calibrate Laser").join("notes.txt"),
        "keep the experiment notes",
    )
    .expect("task user file should be written");

    let result =
        spielgantt_lib::app_facade::rename_task(&project_dir, "Calibrate Laser", "calibrate laser")
            .expect("case-only application task rename should succeed");

    assert!(
        result
            .project()
            .tasks()
            .iter()
            .any(|task| task.id() == "calibrate laser"),
        "refreshed project should report the renamed task id"
    );
    let renamed_metadata =
        read_task_metadata(project_dir.join("calibrate laser/.spielgantt/task.json"));
    assert_eq!(
        renamed_metadata.id, "calibrate laser",
        "case-only rename should update the stored task metadata"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("calibrate laser/notes.txt"))
            .expect("renamed task user file should remain readable"),
        "keep the experiment notes"
    );
    assert_eq!(
        direct_child_names(&project_dir),
        vec![".spielgantt".to_string(), "calibrate laser".to_string()],
        "case-only rename should update the on-disk folder basename instead of reporting a false collision"
    );
}

#[test]
fn app_task_rename_keeps_metadata_and_dependencies_unchanged_when_folder_rename_fails() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written");

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let dependency_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    let original_task_metadata =
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");
    let original_dependency_metadata = std::fs::read_to_string(&dependency_metadata_path)
        .expect("dependency metadata should be readable");
    let overlong_name = "a".repeat(260);

    let error =
        spielgantt_lib::app_facade::rename_task(&project_dir, "calibrate-laser", &overlong_name)
            .expect_err("folder rename failure should bubble up");

    assert!(
        error.to_string().contains("failed to rename task folder"),
        "late rename failures should still report the folder rename failure: {error:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_task_metadata,
        "folder rename failure should leave the renamed task metadata unchanged"
    );
    assert_eq!(
        std::fs::read_to_string(&dependency_metadata_path)
            .expect("dependency metadata should remain readable"),
        original_dependency_metadata,
        "folder rename failure should not rewrite downstream dependency references"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "folder rename failure should leave the original task folder in place"
    );
}

#[test]
fn app_task_dependency_remove_persists_and_refreshes_project_relationships() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("blocker task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    spielgantt_lib::task::add_dependency(&project_dir, "analyze-results", "calibrate-laser")
        .expect("fixture dependency should be added");

    let result = spielgantt_lib::app_facade::remove_task_dependency(
        &project_dir,
        "analyze-results",
        "calibrate-laser",
    )
    .expect("application dependency remove should succeed");

    let refreshed_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("refreshed application project should include the dependent task");
    assert!(
        refreshed_task.dependencies().is_empty(),
        "removed dependency should disappear from refreshed application project state"
    );
    let metadata = read_task_metadata(project_dir.join("analyze-results/.spielgantt/task.json"));
    assert!(
        metadata.dependencies.is_empty(),
        "application dependency removal should persist through the shared core dependency writer"
    );
}

#[test]
fn app_task_dependency_remove_cleans_stale_missing_blocker_references() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    let task_metadata_path = project_dir.join("analyze-results/.spielgantt/task.json");
    std::fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("fixture stale dependency should be written");

    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should load invalid project state");
    assert!(
        !opened.is_valid(),
        "fixture should start invalid because the blocker task is missing"
    );

    let result = spielgantt_lib::app_facade::remove_task_dependency(
        &project_dir,
        "analyze-results",
        "calibrate-laser",
    )
    .expect("application dependency remove should clean up a stale missing blocker reference");

    assert!(
        result.project().is_valid(),
        "removing the stale dependency should refresh to a valid project"
    );
    let refreshed_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("refreshed application project should include the dependent task");
    assert!(
        refreshed_task.dependencies().is_empty(),
        "stale missing blocker should disappear from refreshed application project state"
    );
    let metadata = read_task_metadata(&task_metadata_path);
    assert!(
        !metadata
            .dependencies
            .contains(&"calibrate-laser".to_string()),
        "application dependency removal should remove the literal missing blocker id"
    );
}

#[test]
fn app_task_dependency_add_rejects_cycles_without_rewriting_metadata() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("first task creation should succeed");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("second task creation should succeed");
    spielgantt_lib::task::add_dependency(&project_dir, "analyze-results", "calibrate-laser")
        .expect("fixture dependency should be added");
    let blocker_metadata_path = project_dir.join("calibrate-laser/.spielgantt/task.json");
    let original_blocker_metadata =
        std::fs::read_to_string(&blocker_metadata_path).expect("metadata should be readable");

    let result = spielgantt_lib::app_facade::add_task_dependency(
        &project_dir,
        "calibrate-laser",
        "analyze-results",
    );

    assert!(
        result
            .expect_err("application dependency add should reject cycles")
            .to_string()
            .contains("dependency cycle"),
        "cycle rejection should surface the shared core dependency error"
    );
    assert_eq!(
        std::fs::read_to_string(&blocker_metadata_path).expect("metadata should remain readable"),
        original_blocker_metadata,
        "rejected application dependency add should not rewrite task metadata"
    );
}

#[test]
fn app_task_edit_updates_metadata_and_readme_and_rejects_stale_readme_writes() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("fixture task creation should succeed");
    let readme_path = project_dir.join("calibrate-laser/README.md");
    std::fs::write(
        &readme_path,
        "# Calibrate laser\n\n- keep ordinary markdown\n",
    )
    .expect("fixture README should be writable");
    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include editable task data");
    let task = opened
        .tasks()
        .iter()
        .find(|task| task.id() == "calibrate-laser")
        .expect("opened project should include the task");

    let result = spielgantt_lib::app_facade::edit_task(
        &project_dir,
        "calibrate-laser",
        spielgantt_lib::app_facade::TaskEdit {
            status: Some("unblocked".to_string()),
            readme_content: "# Calibrate laser\n\n- aligned optics\n".to_string(),
            expected_readme_version: task.readme_version().to_string(),
        },
    )
    .expect("application task edit should succeed with current README state");

    let edited_task = result
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "calibrate-laser")
        .expect("refreshed application project should include the edited task");
    assert_eq!(
        edited_task.status(),
        Some(&spielgantt_lib::metadata::TaskStatus::Unblocked)
    );
    let metadata = read_task_metadata(project_dir.join("calibrate-laser/.spielgantt/task.json"));
    assert_eq!(
        metadata.status,
        Some(spielgantt_lib::metadata::TaskStatus::Unblocked),
        "application edit should persist task metadata through the shared core JSON writer"
    );
    assert_eq!(
        std::fs::read_to_string(&readme_path).expect("README should be readable"),
        "# Calibrate laser\n\n- aligned optics\n",
        "README editing should preserve ordinary Markdown exactly"
    );

    let stale_version = edited_task.readme_version().to_string();
    std::fs::write(&readme_path, "# Calibrate laser\n\nexternal update\n")
        .expect("external README edit should be writable");
    let stale_result = spielgantt_lib::app_facade::edit_task(
        &project_dir,
        "calibrate-laser",
        spielgantt_lib::app_facade::TaskEdit {
            status: Some("done".to_string()),
            readme_content: "# Calibrate laser\n\nstale application text\n".to_string(),
            expected_readme_version: stale_version,
        },
    );

    assert!(
        stale_result.is_err(),
        "application task edit should reject stale README writes"
    );
    assert_eq!(
        std::fs::read_to_string(&readme_path).expect("README should remain readable"),
        "# Calibrate laser\n\nexternal update\n",
        "stale application state should not silently overwrite external file changes"
    );
}

#[test]
fn app_task_edit_rolls_back_metadata_when_readme_write_fails() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("fixture task creation should succeed");
    let metadata_path = project_dir.join("calibrate-laser/.spielgantt/task.json");
    let original_metadata =
        std::fs::read_to_string(&metadata_path).expect("task metadata should be readable");
    let readme_path = project_dir.join("calibrate-laser/README.md");
    std::fs::remove_file(&readme_path).expect("fixture README file should be removable");
    std::fs::create_dir(&readme_path).expect("fixture README path should become unwritable");
    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should tolerate a missing README file");
    let task = opened
        .tasks()
        .iter()
        .find(|task| task.id() == "calibrate-laser")
        .expect("opened project should include the task");

    let result = spielgantt_lib::app_facade::edit_task(
        &project_dir,
        "calibrate-laser",
        spielgantt_lib::app_facade::TaskEdit {
            status: Some("unblocked".to_string()),
            readme_content: "# Calibrate laser\n\n- aligned optics\n".to_string(),
            expected_readme_version: task.readme_version().to_string(),
        },
    );

    let error = result.expect_err("README write failure should reject the application task edit");
    assert!(
        error.to_string().contains("failed to write task README"),
        "task edit should surface the original README write failure: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(&metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "failed README writes should roll back earlier metadata rewrites"
    );
}

#[test]
fn app_task_create_refreshes_the_open_project_task_list() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let result = spielgantt_lib::app_facade::create_task(&project_dir, "calibrate-laser")
        .expect("application task creation should succeed");

    assert!(
        result
            .project()
            .tasks()
            .iter()
            .any(|task| task.id() == "calibrate-laser"),
        "created task should appear in the refreshed application project: {result:?}"
    );
    assert!(
        project_dir
            .join("calibrate-laser/.spielgantt/task.json")
            .is_file(),
        "application task creation should use the shared core task creation path"
    );
}

#[test]
fn app_task_adopt_refreshes_the_open_project_task_list_without_rewriting_user_files() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    let notes_dir = project_dir.join("analysis notes");
    std::fs::create_dir(&notes_dir).expect("existing folder should be created");
    std::fs::write(notes_dir.join("notes.txt"), "calibration checklist")
        .expect("user file should be written");

    let result =
        spielgantt_lib::app_facade::adopt_task(&project_dir, &notes_dir, "calibrate-laser")
            .expect("application task adoption should succeed");

    assert!(
        result
            .project()
            .tasks()
            .iter()
            .any(|task| {
                task.id() == "calibrate-laser" && task.project_relative_path() == "analysis notes"
            }),
        "adopted task should appear at its existing folder in the refreshed application project: {result:?}"
    );
    assert_eq!(
        std::fs::read_to_string(notes_dir.join("notes.txt"))
            .expect("user file should remain readable"),
        "calibration checklist",
        "application adoption should preserve ordinary user files"
    );
}

#[test]
fn app_task_adoptable_folders_uses_the_shared_core_candidate_list() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::create_dir(project_dir.join("zeta notes"))
        .expect("candidate folder should be created");
    std::fs::create_dir(project_dir.join("Alpha Samples"))
        .expect("candidate folder should be created");
    spielgantt_lib::task::create(&project_dir, "existing-task")
        .expect("existing task should be created");

    let candidates = spielgantt_lib::app_facade::list_adoptable_task_folders(&project_dir)
        .expect("application adapter should list adoptable task folders");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].project_relative_path(), "Alpha Samples");
    assert_eq!(candidates[0].task_id(), "Alpha Samples");
    assert_eq!(
        candidates[0]
            .folder_path()
            .canonicalize()
            .expect("candidate path should canonicalize"),
        project_dir
            .join("Alpha Samples")
            .canonicalize()
            .expect("expected candidate path should canonicalize"),
    );
    assert_eq!(candidates[1].project_relative_path(), "zeta notes");
    assert_eq!(candidates[1].task_id(), "zeta notes");
}

#[test]
fn app_task_adopt_rejects_folders_outside_the_open_project_without_writing_metadata() {
    let workspace = tempdir().expect("workspace should be created");
    let open_project_dir = workspace.path().join("open-project");
    let other_project_dir = workspace.path().join("other-project");
    std::fs::create_dir(&open_project_dir).expect("open project directory should be created");
    std::fs::create_dir(&other_project_dir).expect("other project directory should be created");
    spielgantt_lib::project::init(&open_project_dir).expect("open project init should succeed");
    spielgantt_lib::project::init(&other_project_dir).expect("other project init should succeed");
    let other_notes_dir = other_project_dir.join("analysis notes");
    std::fs::create_dir(&other_notes_dir).expect("other project folder should be created");
    std::fs::write(other_notes_dir.join("notes.txt"), "do not adopt from here")
        .expect("other project user file should be written");

    let result = spielgantt_lib::app_facade::adopt_task(
        &open_project_dir,
        &other_notes_dir,
        "calibrate-laser",
    );

    assert!(
        result
            .expect_err("application adoption should reject folders outside the open project")
            .to_string()
            .contains("not inside the open project"),
        "cross-project adoption should explain the open-project boundary"
    );
    assert!(
        !other_notes_dir.join(".spielgantt/task.json").exists(),
        "rejected cross-project adoption should not write metadata into the other project"
    );
    assert!(
        spielgantt_lib::app_facade::open_project(&open_project_dir)
            .expect("open project should remain readable")
            .tasks()
            .is_empty(),
        "rejected cross-project adoption should not add tasks to the open project"
    );
}

#[test]
fn app_task_normalize_previews_then_applies_folder_renames_and_refreshes_project_paths() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    let notes_dir = project_dir.join("analysis notes");
    std::fs::create_dir(&notes_dir).expect("existing folder should be created");
    std::fs::write(notes_dir.join("notes.txt"), "calibration checklist")
        .expect("user file should be written");
    spielgantt_lib::task::adopt(&notes_dir, "calibrate-laser")
        .expect("fixture task adoption should succeed");
    let canonical_notes_dir =
        std::fs::canonicalize(&notes_dir).expect("notes directory should have a canonical path");
    let canonical_project_dir =
        std::fs::canonicalize(&project_dir).expect("project path should canonicalize");
    let canonical_normalized_dir = canonical_project_dir.join("calibrate-laser");

    let preview = spielgantt_lib::app_facade::preview_task_normalization(&project_dir)
        .expect("application normalization preview should succeed");

    assert!(!preview.applied(), "preview should be a dry run");
    assert!(
        preview.issues().is_empty(),
        "safe normalization preview should have no preflight issues: {preview:?}"
    );
    assert_eq!(preview.renames().len(), 1);
    assert_eq!(preview.renames()[0].id(), "calibrate-laser");
    assert_eq!(preview.renames()[0].from(), canonical_notes_dir.as_path());
    assert_eq!(
        preview.renames()[0].to(),
        canonical_normalized_dir.as_path()
    );
    assert!(
        notes_dir.is_dir(),
        "normalization preview should not rename the existing task folder"
    );

    let applied = spielgantt_lib::app_facade::apply_task_normalization(&project_dir)
        .expect("application normalization apply should succeed");

    assert!(
        applied.applied(),
        "apply result should mark normalization as applied"
    );
    assert!(
        project_dir.join("calibrate-laser/notes.txt").is_file(),
        "normalization apply should preserve user files at the renamed folder"
    );
    assert!(
        applied.project().tasks().iter().any(|task| {
            task.id() == "calibrate-laser" && task.project_relative_path() == "calibrate-laser"
        }),
        "applied normalization should refresh application project paths: {applied:?}"
    );
}
