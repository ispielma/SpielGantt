use tempfile::tempdir;

mod support;

use support::{create_task, run_spielgantt_in, stderr_text, stdout_json};

fn assert_task_bucket_shape(project_dir: &std::path::Path, task_id: &str) {
    let task_dir = project_dir.join(task_id);
    assert!(task_dir.is_dir(), "task bucket should be a directory");
    assert!(
        task_dir.join(".spielgantt").is_dir(),
        "task bucket should contain a hidden metadata directory"
    );
    assert!(
        task_dir.join(".spielgantt/task.json").is_file(),
        "task bucket should contain task JSON metadata"
    );
    assert_eq!(
        std::fs::read_to_string(task_dir.join("README.md"))
            .expect("task README should be readable"),
        format!("# {task_id}\n"),
        "task bucket should contain the same README heading shape"
    );

    let mut relative_entries = std::fs::read_dir(&task_dir)
        .expect("task bucket entries should be readable")
        .map(|entry| {
            entry
                .expect("task bucket entry should be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    relative_entries.sort();
    assert_eq!(
        relative_entries,
        vec![".spielgantt".to_string(), "README.md".to_string()],
        "task bucket should only contain user-facing README and hidden metadata at creation"
    );
}

fn read_task_metadata(
    project_dir: &std::path::Path,
    task_id: &str,
) -> spielgantt_lib::metadata::TaskMetadata {
    spielgantt_lib::metadata::TaskMetadata::from_json(
        &std::fs::read_to_string(project_dir.join(task_id).join(".spielgantt/task.json"))
            .expect("task metadata should be readable"),
    )
    .expect("task metadata should remain valid JSON")
}

#[test]
fn task_create_and_relative_insert_create_the_same_task_bucket_shape() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before comparing task bucket shape: {init_output:?}"
    );

    let create_output = create_task(&project_dir, "ordinary-task");
    assert!(
        create_output.status.success(),
        "ordinary task create should succeed: {create_output:?}"
    );
    let selected_output = create_task(&project_dir, "selected-task");
    assert!(
        selected_output.status.success(),
        "selected fixture task should be created before relative insert: {selected_output:?}"
    );
    let insert_output = run_spielgantt_in(
        &project_dir,
        &["task", "insert-after", "selected-task", "inserted-task"],
    );
    assert!(
        insert_output.status.success(),
        "relative task insert should succeed: {insert_output:?}"
    );

    assert_task_bucket_shape(&project_dir, "ordinary-task");
    assert_task_bucket_shape(&project_dir, "inserted-task");
}

#[test]
fn task_insert_before_json_transfers_selected_blockers_to_the_new_task() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before relative insert: {init_output:?}"
    );
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");
    assert!(create_task(&project_dir, "prepare-inputs").status.success());
    assert!(create_task(&project_dir, "analysis-run").status.success());
    std::fs::write(
        project_dir.join("analysis-run/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-inputs\"\n  ]\n}\n",
    )
    .expect("selected task metadata should be rewritten with blockers");

    let insert_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-before",
            "analysis-run",
            "quality-gate",
            "--json",
        ],
    );

    assert!(
        insert_output.status.success(),
        "task insert-before should succeed: {insert_output:?}"
    );
    let json = stdout_json(&insert_output);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["mode"], "before");
    assert_eq!(json["selected_task_id"], "analysis-run");
    assert_eq!(json["inserted_task_id"], "quality-gate");

    let inserted_metadata = read_task_metadata(&project_dir, "quality-gate");
    assert_eq!(
        inserted_metadata.dependencies,
        vec!["START".to_string(), "prepare-inputs".to_string()],
        "inserted task should inherit the selected task blockers"
    );
    let selected_metadata = read_task_metadata(&project_dir, "analysis-run");
    assert_eq!(
        selected_metadata.dependencies,
        vec!["quality-gate".to_string()],
        "selected task should be rewired to depend only on the inserted predecessor"
    );
}

#[test]
fn task_insert_after_json_rewires_downstream_tasks_to_the_new_successor() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before relative insert: {init_output:?}"
    );
    assert!(create_task(&project_dir, "analysis-run").status.success());
    assert!(create_task(&project_dir, "write-report").status.success());
    std::fs::write(
        project_dir.join("write-report/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"write-report\",\n  \"dependencies\": [\n    \"analysis-run\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be rewritten with the selected blocker");

    let insert_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-after",
            "analysis-run",
            "archive-results",
            "--json",
        ],
    );

    assert!(
        insert_output.status.success(),
        "task insert-after should succeed: {insert_output:?}"
    );
    let json = stdout_json(&insert_output);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["mode"], "after");
    assert_eq!(json["selected_task_id"], "analysis-run");
    assert_eq!(json["inserted_task_id"], "archive-results");

    let inserted_metadata = read_task_metadata(&project_dir, "archive-results");
    assert_eq!(
        inserted_metadata.dependencies,
        vec!["analysis-run".to_string()],
        "inserted task should depend on the selected predecessor"
    );
    let downstream_metadata = read_task_metadata(&project_dir, "write-report");
    assert_eq!(
        downstream_metadata.dependencies,
        vec!["archive-results".to_string()],
        "downstream tasks should be rewired to the inserted successor"
    );
}

#[test]
fn task_insert_after_transfers_end_event_and_clears_the_selected_task() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before relative insert: {init_output:?}"
    );
    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"Analysis complete\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");
    assert!(create_task(&project_dir, "analysis-run").status.success());
    assert!(create_task(&project_dir, "write-report").status.success());
    std::fs::write(
        project_dir.join("analysis-run/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"ends_at\": \"Analysis complete\"\n}\n",
    )
    .expect("selected task metadata should be rewritten with an end event");
    std::fs::write(
        project_dir.join("write-report/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"write-report\",\n  \"dependencies\": [\n    \"analysis-run\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be rewritten with the selected blocker");

    let insert_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-after",
            "analysis-run",
            "archive-results",
            "--json",
        ],
    );

    assert!(
        insert_output.status.success(),
        "task insert-after should succeed: {insert_output:?}"
    );
    let selected_metadata = read_task_metadata(&project_dir, "analysis-run");
    assert_eq!(
        selected_metadata.ends_at, None,
        "selected task should no longer own the transferred end event"
    );
    let inserted_metadata = read_task_metadata(&project_dir, "archive-results");
    assert_eq!(
        inserted_metadata.ends_at,
        Some("Analysis complete".to_string()),
        "inserted task should inherit the selected end event"
    );
    assert_eq!(
        inserted_metadata.dependencies,
        vec!["analysis-run".to_string()],
        "inserted task should still follow the selected task"
    );
    let downstream_metadata = read_task_metadata(&project_dir, "write-report");
    assert_eq!(
        downstream_metadata.dependencies,
        vec!["Analysis complete".to_string()],
        "downstream tasks should depend on the transferred end event instead of a task ending at that event"
    );
}

#[test]
fn task_relative_insert_rejects_invalid_selected_and_duplicate_ids_before_writing() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before relative insert rejection checks: {init_output:?}"
    );
    assert!(create_task(&project_dir, "analysis-run").status.success());

    let missing_selected_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-before",
            "missing-task",
            "quality-gate",
            "--json",
        ],
    );
    assert!(
        !missing_selected_output.status.success(),
        "missing selected task should be rejected"
    );
    assert!(
        stderr_text(&missing_selected_output).contains("task id 'missing-task' was not found"),
        "missing-selected rejection should identify the selected task: {}",
        stderr_text(&missing_selected_output)
    );
    assert!(
        !project_dir.join("quality-gate").exists(),
        "missing-selected rejection should not create the inserted task folder"
    );

    assert!(create_task(&project_dir, "existing-task").status.success());
    let selected_metadata_path = project_dir.join("analysis-run/.spielgantt/task.json");
    let original_selected_metadata = std::fs::read_to_string(&selected_metadata_path)
        .expect("selected task metadata should be readable");
    let duplicate_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-after",
            "analysis-run",
            "existing-task",
            "--json",
        ],
    );
    assert!(
        !duplicate_output.status.success(),
        "duplicate inserted task id should be rejected"
    );
    assert!(
        stderr_text(&duplicate_output).contains("task id 'existing-task' already exists"),
        "duplicate rejection should identify the inserted task id: {}",
        stderr_text(&duplicate_output)
    );
    assert_eq!(
        std::fs::read_to_string(&selected_metadata_path)
            .expect("selected task metadata should remain readable"),
        original_selected_metadata,
        "duplicate rejection should not rewrite selected task metadata"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_insert_after_rolls_back_existing_metadata_and_removes_inserted_task() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace.path(), &["init", "experiment-plan"]);
    assert!(
        init_output.status.success(),
        "init should succeed before relative insert rollback check: {init_output:?}"
    );
    assert!(create_task(&project_dir, "analysis-run").status.success());
    assert!(create_task(&project_dir, "write-report").status.success());
    std::fs::write(
        project_dir.join("write-report/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"write-report\",\n  \"dependencies\": [\n    \"analysis-run\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be rewritten with the selected blocker");

    let selected_metadata_path = project_dir.join("analysis-run/.spielgantt/task.json");
    let selected_metadata_before = std::fs::read_to_string(&selected_metadata_path)
        .expect("selected metadata should be readable before failed insert");
    let downstream_metadata_dir = project_dir.join("write-report/.spielgantt");
    let downstream_metadata_path = downstream_metadata_dir.join("task.json");
    let downstream_metadata_before = std::fs::read_to_string(&downstream_metadata_path)
        .expect("downstream metadata should be readable before failed insert");

    let downstream_metadata_permissions = std::fs::metadata(&downstream_metadata_dir)
        .expect("downstream metadata directory permissions should be readable")
        .permissions()
        .mode();
    std::fs::set_permissions(
        &downstream_metadata_dir,
        std::fs::Permissions::from_mode(downstream_metadata_permissions & !0o222),
    )
    .expect("downstream metadata directory should be made read-only");

    let insert_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "insert-after",
            "analysis-run",
            "archive-results",
            "--json",
        ],
    );

    std::fs::set_permissions(
        &downstream_metadata_dir,
        std::fs::Permissions::from_mode(downstream_metadata_permissions),
    )
    .expect("downstream metadata directory permissions should be restored");

    assert!(
        !insert_output.status.success(),
        "task insert-after should fail when downstream metadata cannot be rewritten: {insert_output:?}"
    );
    assert!(
        stderr_text(&insert_output).contains("failed to write temporary task metadata"),
        "insert failure should report the metadata write failure: {}",
        stderr_text(&insert_output)
    );
    assert!(
        !project_dir.join("archive-results").exists(),
        "failed relative insert should remove the newly created task bucket"
    );
    assert_eq!(
        std::fs::read_to_string(&selected_metadata_path)
            .expect("selected metadata should remain readable after failed insert"),
        selected_metadata_before,
        "failed relative insert should roll back selected task metadata"
    );
    assert_eq!(
        std::fs::read_to_string(&downstream_metadata_path)
            .expect("downstream metadata should remain readable after failed insert"),
        downstream_metadata_before,
        "failed relative insert should leave downstream metadata unchanged"
    );
}
