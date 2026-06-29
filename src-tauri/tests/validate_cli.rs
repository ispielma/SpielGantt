use std::fs;

mod support;

use tempfile::tempdir;

#[test]
fn validate_accepts_a_structurally_valid_project() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    let create_output = support::create_task(&project_dir, "RbK New Experiment");
    assert!(
        create_output.status.success(),
        "task create should succeed before validate: {create_output:?}"
    );

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "validate should succeed for a valid project: {validate_output:?}"
    );
    assert!(
        support::stdout_text(&validate_output).contains("project is valid"),
        "validate should confirm success: {}",
        support::stdout_text(&validate_output)
    );
}

#[test]
fn validate_accepts_distinct_task_and_event_ids() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    let create_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        create_output.status.success(),
        "task create should succeed when the task id does not collide with an event id: {create_output:?}"
    );

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "validate should accept a project whose task ids do not overlap with events: {validate_output:?}"
    );
}

#[test]
fn validate_reports_task_event_collisions() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("calibrate-laser"), "calibrate-laser");
    support::write_task_metadata(&project_dir.join("collision"), "MOT");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail when a task id collides with an event id: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains("task id 'MOT' collides with project event id 'MOT'"),
        "validate should explain the task-event collision: {validation_errors}"
    );
}

#[test]
fn validate_accepts_task_dependencies_and_ends_at_that_target_existing_events() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before dependency validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("task metadata should be rewritten with an event dependency and ends_at");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "validate should accept dependencies that target existing events: {validate_output:?}"
    );
}

#[test]
fn validate_reports_missing_task_or_event_references() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before missing-reference validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"MISSING-BLOCKER\"\n  ],\n  \"ends_at\": \"MISSING-EVENT\"\n}\n",
    )
    .expect("task metadata should be rewritten with missing references");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail when dependencies or ends_at reference missing ids: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains(
            "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'"
        ),
        "validate should report the missing dependency target: {validation_errors}"
    );
    assert!(
        validation_errors
            .contains("task 'analyze-results' ends_at references missing event id 'MISSING-EVENT'"),
        "validate should report the missing ends_at event: {validation_errors}"
    );
}

#[test]
fn validate_reports_ends_at_references_to_tasks() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before ends_at validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\"\n}\n",
    )
    .expect("task metadata should be written for the reference task");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"ends_at\": \"prepare-sample\"\n}\n",
    )
    .expect("task metadata should be rewritten with an invalid ends_at reference");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail when ends_at targets a task id: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains(
            "task 'analyze-results' ends_at must reference an event id, not task id 'prepare-sample'"
        ),
        "validate should explain that ends_at must target an event: {validation_errors}"
    );
}

#[test]
fn validate_reports_dependencies_on_tasks_that_end_at_events() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before dependency-target validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("task metadata should be rewritten with ends_at");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-sample\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be rewritten with a task dependency");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should reject dependencies on tasks that end at events: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains(
            "task 'analyze-results' depends on task 'prepare-sample' which ends at event 'BEC'; depend on the event instead"
        ),
        "validate should explain that the dependency should target the event: {validation_errors}"
    );
}

#[test]
fn validate_json_resolves_nested_start_path_and_reports_duplicate_task_ids() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before nested duplicate validation: {init_output:?}"
    );

    fs::create_dir(project_dir.join("nested")).expect("nested task parent should be created");
    support::write_task_metadata(&project_dir.join("calibrate-laser"), "calibrate-laser");
    support::write_task_metadata(&project_dir.join("nested/task-copy"), "analyze-results");

    let nested_start = project_dir.join("nested/task-copy");
    let task_show_json = support::run_spielgantt_json_in(
        &nested_start,
        &["task", "show", "--json", "calibrate-laser"],
    );
    assert_eq!(task_show_json["schema_version"], 1);
    assert_eq!(task_show_json["id"], "calibrate-laser");
    assert_eq!(
        task_show_json["path"],
        fs::canonicalize(project_dir.join("calibrate-laser"))
            .expect("task path should canonicalize")
            .display()
            .to_string()
    );

    fs::write(
        nested_start.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("task metadata should be rewritten with a duplicate id");

    let validate_output = support::run_spielgantt_in(&nested_start, &["validate", "--json"]);
    assert!(
        !validate_output.status.success(),
        "validate --json should fail for duplicate task ids from a nested start path: {validate_output:?}"
    );
    assert!(
        support::stderr_text(&validate_output).is_empty(),
        "validate --json should keep diagnostics in stdout JSON"
    );

    let validate_json = support::stdout_json(&validate_output);
    assert_eq!(validate_json["schema_version"], 1);
    assert_eq!(validate_json["valid"], false);
    assert_eq!(
        validate_json["project_root"],
        fs::canonicalize(&project_dir)
            .expect("project directory should canonicalize")
            .display()
            .to_string()
    );
    let issues = support::json_array(&validate_json, "issues");
    assert_eq!(
        issues.len(),
        1,
        "duplicate task ids should be reported exactly once: {validate_json}"
    );
    let diagnostic = issues[0].as_str().expect("issue should be a string");
    assert!(
        diagnostic.contains("duplicate task id 'calibrate-laser'"),
        "duplicate diagnostic should name the conflicting task id: {diagnostic}"
    );
    assert!(
        diagnostic.contains("calibrate-laser") && diagnostic.contains("nested/task-copy"),
        "duplicate diagnostic should include both task package paths: {diagnostic}"
    );
}

#[test]
fn validate_reports_cycles_that_traverse_events() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before cycle validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MID\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("task-a"), "task-a");
    fs::write(
        project_dir.join("task-a").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"task-a\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"MID\"\n}\n",
    )
    .expect("first task metadata should be rewritten");

    support::write_task_metadata(&project_dir.join("task-b"), "task-b");
    fs::write(
        project_dir.join("task-b").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"task-b\",\n  \"dependencies\": [\n    \"MID\"\n  ],\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("second task metadata should be rewritten");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for a cycle that traverses events: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors
            .contains("dependency cycle detected: task-a -> START -> task-b -> MID -> task-a")
            || validation_errors
                .contains("dependency cycle detected: task-b -> MID -> task-a -> START -> task-b"),
        "validate should report the cycle path through events: {validation_errors}"
    );
}

#[test]
fn validate_rejects_a_directory_without_project_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let notes_dir = workspace_dir.path().join("notes");
    fs::create_dir(&notes_dir).expect("notes directory should be created");

    let validate_output = support::run_spielgantt_in(&notes_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail outside a SpielGantt project: {validate_output:?}"
    );
    assert!(
        support::stderr_text(&validate_output).contains("missing project metadata"),
        "validate should explain the missing project metadata: {}",
        support::stderr_text(&validate_output)
    );
}

#[test]
fn validate_reports_duplicate_ids_and_malformed_task_metadata_in_one_run() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let malformed_task_dir = project_dir.join("broken-task");
    let duplicate_task_dir = project_dir.join("duplicate-adopted-task");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    let create_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        create_output.status.success(),
        "task create should succeed before validate: {create_output:?}"
    );

    fs::create_dir(&duplicate_task_dir).expect("duplicate task directory should be created");
    let adopt_output = support::run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "adopt",
            "duplicate-adopted-task",
            "--id",
            "prepare-sample",
        ],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed before tampering with metadata: {adopt_output:?}"
    );

    fs::write(
        duplicate_task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("duplicate task metadata should be rewritten");

    fs::create_dir(&malformed_task_dir).expect("malformed task directory should be created");
    fs::create_dir(malformed_task_dir.join(".spielgantt"))
        .expect("malformed metadata directory should be created");
    fs::write(
        malformed_task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": [\n    \"not-a-string\"\n  ]\n}\n",
    )
    .expect("malformed task metadata should be written");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for an invalid project: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains("duplicate task id 'calibrate-laser'"),
        "validate should report duplicate ids: {validation_errors}"
    );
    assert!(
        validation_errors.contains("failed to parse task metadata"),
        "validate should report malformed task metadata: {validation_errors}"
    );
}

#[test]
fn validate_reports_readable_task_diagnostics_when_another_task_is_malformed() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let malformed_task_dir = project_dir.join("broken-task");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"missing-blocker\"\n  ]\n}\n",
    )
    .expect("readable task metadata should be rewritten with a missing dependency");

    fs::create_dir(&malformed_task_dir).expect("malformed task directory should be created");
    fs::create_dir(malformed_task_dir.join(".spielgantt"))
        .expect("malformed metadata directory should be created");
    fs::write(
        malformed_task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": [\n    \"not-a-string\"\n  ]\n}\n",
    )
    .expect("malformed task metadata should be written");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for an invalid project: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains("failed to parse task metadata"),
        "validate should report malformed task metadata: {validation_errors}"
    );
    assert!(
        validation_errors.contains(
            "task 'analyze-results' depends on missing task or event id 'missing-blocker'"
        ),
        "validate should keep diagnostics from readable tasks: {validation_errors}"
    );
}

#[cfg(unix)]
#[test]
fn validate_reports_readable_task_diagnostics_when_another_task_metadata_cannot_be_read() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let unreadable_task_dir = project_dir.join("unreadable-task");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"missing-blocker\"\n  ]\n}\n",
    )
    .expect("readable task metadata should be rewritten with a missing dependency");

    support::write_task_metadata(&unreadable_task_dir, "unreadable-task");
    let unreadable_metadata_path = unreadable_task_dir.join(".spielgantt/task.json");
    let mut unreadable_permissions = fs::metadata(&unreadable_metadata_path)
        .expect("unreadable task metadata should have metadata")
        .permissions();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&unreadable_metadata_path, unreadable_permissions)
        .expect("task metadata should be made unreadable");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);

    let mut restored_permissions = fs::metadata(&unreadable_metadata_path)
        .expect("unreadable task metadata should still have metadata")
        .permissions();
    restored_permissions.set_mode(0o600);
    fs::set_permissions(&unreadable_metadata_path, restored_permissions)
        .expect("task metadata permissions should be restored");

    assert!(
        !validate_output.status.success(),
        "validate should fail for an invalid project: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains("failed to read task metadata"),
        "validate should report unreadable task metadata: {validation_errors}"
    );
    assert!(
        validation_errors.contains(
            "task 'analyze-results' depends on missing task or event id 'missing-blocker'"
        ),
        "validate should keep diagnostics from readable tasks: {validation_errors}"
    );
}

#[test]
fn validate_reports_duplicate_task_and_event_ids_in_one_run() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let first_task_dir = project_dir.join("phase-1");
    let second_task_dir = project_dir.join("phase-2");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before validate: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"START\"\n  ]\n}\n",
    );

    support::write_task_metadata(&first_task_dir, "calibrate-laser");
    support::write_task_metadata(&second_task_dir, "calibrate-laser");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail when both task and event namespaces contain duplicates: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains("duplicate event id 'START'"),
        "validate should report the duplicate event id: {validation_errors}"
    );
    assert!(
        validation_errors.contains("duplicate task id 'calibrate-laser'"),
        "validate should report the duplicate task id: {validation_errors}"
    );
}

#[test]
fn validate_reports_dependencies_that_point_to_missing_task_ids() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before missing dependency validation: {init_output:?}"
    );

    let create_output = support::create_task(&project_dir, "analyze-results");
    assert!(
        create_output.status.success(),
        "task create should succeed before missing dependency validation: {create_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("task metadata should be rewritten with a missing dependency");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail when a dependency points to a missing task id: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains(
            "task 'analyze-results' depends on missing task or event id 'calibrate-laser'"
        ),
        "validate should report the missing dependency id: {validation_errors}"
    );
}

#[test]
fn validate_reports_dependency_cycles() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before cycle validation: {init_output:?}"
    );

    let create_first_output = support::create_task(&project_dir, "analyze-results");
    assert!(
        create_first_output.status.success(),
        "first task create should succeed before cycle validation: {create_first_output:?}"
    );
    let create_second_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        create_second_output.status.success(),
        "second task create should succeed before cycle validation: {create_second_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("first task metadata should be rewritten with a dependency");
    fs::write(
        project_dir
            .join("calibrate-laser")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\",\n  \"dependencies\": [\n    \"analyze-results\"\n  ]\n}\n",
    )
    .expect("second task metadata should be rewritten with a dependency");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for dependency cycles: {validate_output:?}"
    );

    let validation_errors = support::stderr_text(&validate_output);
    assert!(
        validation_errors.contains(
            "dependency cycle detected: analyze-results -> calibrate-laser -> analyze-results"
        ),
        "validate should report the cycle path: {validation_errors}"
    );
}

#[test]
fn validate_ignores_stale_task_links_metadata_without_rewriting_it() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before stale-link validation: {init_output:?}"
    );

    let create_output = support::create_task(&project_dir, "analyze-results");
    assert!(
        create_output.status.success(),
        "task create should succeed before stale-link validation: {create_output:?}"
    );

    let links_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/links.json");
    fs::write(
        &links_metadata_path,
        "stale links metadata should not participate in validation",
    )
    .expect("stale links file should be written");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "validate should ignore stale task link metadata: {validate_output:?}"
    );

    assert_eq!(
        fs::read_to_string(&links_metadata_path)
            .expect("stale links metadata should remain readable after validation"),
        "stale links metadata should not participate in validation",
        "validate should not delete or rewrite stale links metadata"
    );
}
