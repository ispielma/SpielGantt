use std::fs;

use tempfile::tempdir;

mod support;

fn read_task_metadata(path: impl AsRef<std::path::Path>) -> spielgantt_lib::metadata::TaskMetadata {
    spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(path.as_ref()).expect("task metadata should be readable"),
    )
    .expect("task metadata should remain valid JSON")
}

#[test]
fn app_project_onboard_initializes_project_and_adopts_direct_child_tasks_idempotently() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    let sample_task_dir = project_dir.join("Sample Task");
    let notes_task_dir = project_dir.join("notes");
    let hidden_dir = project_dir.join(".hidden");

    fs::create_dir(&project_dir).expect("project directory should be created");
    fs::create_dir(&sample_task_dir).expect("sample task directory should be created");
    fs::create_dir(&notes_task_dir).expect("notes directory should be created");
    fs::create_dir(&hidden_dir).expect("hidden directory should be created");

    fs::create_dir(sample_task_dir.join(".spielgantt"))
        .expect("aligned task metadata directory should be created");
    let aligned_metadata_path = sample_task_dir.join(".spielgantt/task.json");
    let aligned_metadata =
        "{\n  \"schema_version\": 1,\n  \"id\": \"Sample Task\",\n  \"status\": \"planned\"\n}\n";
    fs::write(&aligned_metadata_path, aligned_metadata)
        .expect("aligned task metadata should be written");

    let result = spielgantt_lib::app_facade::onboard_project(&project_dir)
        .expect("application project onboarding should succeed");

    let canonical_project_dir =
        fs::canonicalize(&project_dir).expect("project path should canonicalize");
    assert_eq!(result.selected_path(), project_dir.as_path());
    assert_eq!(result.project_root(), Some(canonical_project_dir.as_path()));
    assert!(result.is_valid());

    let task_ids = result
        .tasks()
        .iter()
        .map(|task| task.id())
        .collect::<Vec<_>>();
    assert_eq!(task_ids, vec!["Sample Task", "notes"]);

    let preserved_metadata = read_task_metadata(&aligned_metadata_path);
    assert_eq!(preserved_metadata.id, "Sample Task");
    assert_eq!(
        preserved_metadata.status,
        Some(spielgantt_lib::metadata::TaskStatus::Unblocked),
        "already aligned task metadata should be preserved"
    );

    let notes_metadata = read_task_metadata(notes_task_dir.join(".spielgantt/task.json"));
    assert_eq!(
        notes_metadata.id, "notes",
        "new task metadata should use the direct child folder name as the task id"
    );

    assert!(
        !hidden_dir.join(".spielgantt/task.json").exists(),
        "hidden directories should be ignored during onboarding"
    );
    support::assert_agent_ready_project(&project_dir);

    let second_result = spielgantt_lib::app_facade::onboard_project(&project_dir)
        .expect("application project onboarding should be idempotent");
    assert_eq!(
        second_result
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        task_ids,
        "re-onboarding should not change the adopted task set"
    );
    let preserved_metadata = read_task_metadata(&aligned_metadata_path);
    assert_eq!(preserved_metadata.id, "Sample Task");
    assert_eq!(
        preserved_metadata.status,
        Some(spielgantt_lib::metadata::TaskStatus::Unblocked),
        "re-onboarding should not rewrite already aligned task metadata"
    );
    support::assert_agent_ready_project(&project_dir);
}

#[test]
fn app_project_onboard_preserves_existing_drifted_task_metadata_for_alignment() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    let drifted_task_dir = project_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    fs::create_dir(&drifted_task_dir).expect("task directory should be created");
    fs::create_dir(drifted_task_dir.join(".spielgantt"))
        .expect("task metadata directory should be created");
    let metadata_path = drifted_task_dir.join(".spielgantt/task.json");
    let drifted_metadata =
        "{\n  \"schema_version\": 1,\n  \"id\": \"Calibrate Laser\",\n  \"status\": \"blocked\"\n}\n";
    fs::write(&metadata_path, drifted_metadata).expect("drifted task metadata should be written");

    let result = spielgantt_lib::app_facade::onboard_project(&project_dir)
        .expect("application project onboarding should preserve drifted task metadata");

    let preserved_metadata = read_task_metadata(&metadata_path);
    assert_eq!(preserved_metadata.id, "Calibrate Laser");
    assert_eq!(
        preserved_metadata.status,
        Some(spielgantt_lib::metadata::TaskStatus::Blocked),
        "onboarding should leave existing canonical task metadata untouched"
    );
    assert!(
        result.tasks().iter().any(|task| {
            task.id() == "Calibrate Laser" && task.project_relative_path() == "analysis notes"
        }),
        "onboarding should expose the metadata id with the current folder path so alignment can report drift: {result:?}"
    );
}
