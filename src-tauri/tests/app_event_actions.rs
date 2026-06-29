use std::fs;

use tempfile::tempdir;

mod support;

#[test]
fn app_event_create_refreshes_the_open_project_snapshot() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");

    let result = spielgantt_lib::app_facade::create_event(&project_dir, "BEC")
        .expect("application event creation should succeed");

    assert!(
        result.project().events().contains(&"BEC".to_string()),
        "created event should appear in the refreshed application project snapshot"
    );
    let metadata = spielgantt_lib::metadata::ProjectMetadata::from_json(
        &std::fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable"),
    )
    .expect("project metadata should remain valid JSON");
    assert!(
        metadata
            .events
            .as_deref()
            .is_some_and(|events| events.contains(&"BEC".to_string())),
        "application event creation should persist the new event through the shared core path"
    );
}

#[test]
fn app_event_rename_refreshes_the_project_snapshot_and_heals_task_references() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    spielgantt_lib::task::create(&project_dir, "prepare-sample")
        .expect("task creation should succeed");
    fs::write(
        project_dir.join("prepare-sample/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("task metadata should be written with ends_at");
    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("dependent task creation should succeed");
    fs::write(
        project_dir.join("analyze-results/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-sample\"\n  ]\n}\n",
    )
    .expect("task metadata should be written with dependencies");

    let result = spielgantt_lib::app_facade::rename_event(&project_dir, "START", "BEGIN")
        .expect("application event rename should succeed");

    assert!(
        result.project().events().contains(&"BEGIN".to_string()),
        "renamed event should appear in the refreshed application project snapshot"
    );
    let ending_task = spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(project_dir.join("prepare-sample/.spielgantt/task.json"))
            .expect("ending task metadata should be readable"),
    )
    .expect("ending task metadata should remain valid JSON");
    assert_eq!(
        ending_task.ends_at.as_deref(),
        Some("BEGIN"),
        "application event rename should heal task ends_at references through the shared core path"
    );
    let dependent_task = spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(project_dir.join("analyze-results/.spielgantt/task.json"))
            .expect("dependent task metadata should be readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert!(
        dependent_task.dependencies.contains(&"BEGIN".to_string()),
        "application event rename should heal task dependency references through the shared core path"
    );
}

#[test]
fn app_event_delete_refreshes_the_open_project_snapshot() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ],\n  \"boundary_events\": {\n    \"start\": \"START\",\n    \"finish\": \"BEC\"\n  }\n}\n",
    )
    .expect("project metadata should be written with events");

    let result = spielgantt_lib::app_facade::delete_event(&project_dir, "MOT")
        .expect("application event deletion should succeed");

    assert!(
        !result.project().events().contains(&"MOT".to_string()),
        "deleted event should be absent from the refreshed application project snapshot"
    );
    let metadata = spielgantt_lib::metadata::ProjectMetadata::from_json(
        &std::fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable"),
    )
    .expect("project metadata should remain valid JSON");
    assert_eq!(
        metadata.events.as_deref(),
        Some(["START".to_string(), "BEC".to_string()].as_slice()),
        "application event deletion should persist through the shared core path"
    );
}
