use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn cache_rebuild_indexes_tasks_from_metadata_and_remains_disposable() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rebuilding the cache: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before rebuilding the cache: {create_output:?}"
    );

    let cache_rebuild_output = run_spielgantt_in(&project_dir, &["cache", "rebuild"]);
    assert!(
        cache_rebuild_output.status.success(),
        "cache rebuild should succeed for a valid project: {cache_rebuild_output:?}"
    );
    assert!(
        stdout(&cache_rebuild_output).contains("Rebuilt cache with 1 task"),
        "cache rebuild should report the indexed task count: {}",
        stdout(&cache_rebuild_output)
    );

    let cache_path = project_dir.join(".spielgantt/cache/tasks.json");
    assert!(
        !project_dir.join(".spielgantt/cache/tasks.yaml").exists(),
        "cache rebuild should not write the old YAML cache file"
    );
    let cache_contents =
        fs::read_to_string(&cache_path).expect("cache rebuild should write a task cache file");
    let cache_metadata: serde_json::Value =
        serde_json::from_str(&cache_contents).expect("cache should be valid JSON");
    assert_eq!(cache_metadata["schema_version"], 1);
    let tasks = support::json_array(&cache_metadata, "tasks");
    assert_eq!(tasks.len(), 1, "cache should index the created task");
    let task = support::find_json_object_by_str(tasks, "id", "calibrate-laser");
    assert_eq!(task["path"], "calibrate-laser");

    fs::remove_file(&cache_path).expect("cache file should be removable");

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "deleting the cache should not invalidate the project: {validate_output:?}"
    );
}

#[test]
fn repair_reports_external_folder_changes_and_invalid_references_without_rewriting_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before repair setup: {init_output:?}"
    );
    for task_id in ["calibrate-laser", "analyze-results", "archive-samples"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }

    let cache_rebuild_output = run_spielgantt_in(&project_dir, &["cache", "rebuild"]);
    assert!(
        cache_rebuild_output.status.success(),
        "cache rebuild should succeed before external edits: {cache_rebuild_output:?}"
    );

    fs::rename(
        project_dir.join("calibrate-laser"),
        project_dir.join("optics-calibration"),
    )
    .expect("task folder should be externally renameable");
    fs::remove_dir_all(project_dir.join("archive-samples"))
        .expect("task folder should be externally removable");

    let analyze_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    fs::write(
        &analyze_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"missing-task\"\n  ]\n}\n",
    )
    .expect("task metadata should be editable outside SpielGantt");
    let original_analyze_metadata =
        fs::read_to_string(&analyze_metadata_path).expect("task metadata should be readable");
    let links_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/links.json");
    fs::write(
        &links_metadata_path,
        "stale links metadata should be ignored by repair",
    )
    .expect("stale links metadata should be written");
    let original_links_metadata =
        fs::read_to_string(&links_metadata_path).expect("links metadata should be readable");

    let repair_output = run_spielgantt_in(&project_dir, &["repair"]);
    assert!(
        !repair_output.status.success(),
        "repair should report unresolved filesystem and reference issues: {repair_output:?}"
    );

    let repair_report = stderr(&repair_output);
    assert!(
        repair_report.contains(
            "task 'calibrate-laser' appears moved or renamed: calibrate-laser -> optics-calibration"
        ),
        "repair should report task folders moved since the last cache rebuild: {repair_report}"
    );
    assert!(
        repair_report.contains("cached task 'archive-samples' is missing from archive-samples"),
        "repair should report task folders missing since the last cache rebuild: {repair_report}"
    );
    assert!(
        repair_report
            .contains("task 'analyze-results' depends on missing task or event id 'missing-task'"),
        "repair should report invalid dependency references: {repair_report}"
    );
    assert!(
        !repair_report.contains("link '"),
        "repair should not treat stale links metadata as package semantics: {repair_report}"
    );

    assert_eq!(
        fs::read_to_string(&analyze_metadata_path)
            .expect("task metadata should remain readable after repair"),
        original_analyze_metadata,
        "repair should not rewrite task metadata by default"
    );
    assert_eq!(
        fs::read_to_string(&links_metadata_path)
            .expect("links metadata should remain readable after repair"),
        original_links_metadata,
        "repair should not rewrite stale links metadata"
    );
}

#[test]
fn repair_uses_validation_wording_once_for_duplicate_task_ids() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before duplicate-id repair setup: {init_output:?}"
    );
    for task_id in ["calibrate-laser", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("task metadata should be editable outside SpielGantt");

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for duplicate task ids: {validate_output:?}"
    );
    let repair_output = run_spielgantt_in(&project_dir, &["repair"]);
    assert!(
        !repair_output.status.success(),
        "repair should fail for duplicate task ids: {repair_output:?}"
    );

    let validation_duplicate_lines = stderr(&validate_output)
        .lines()
        .filter(|line| line.contains("duplicate task id 'calibrate-laser'"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let repair_duplicate_lines = stderr(&repair_output)
        .lines()
        .filter(|line| line.contains("duplicate task id 'calibrate-laser'"))
        .map(str::to_string)
        .collect::<Vec<_>>();

    assert_eq!(
        validation_duplicate_lines.len(),
        1,
        "validate should report the duplicate once: {}",
        stderr(&validate_output)
    );
    assert_eq!(
        repair_duplicate_lines, validation_duplicate_lines,
        "repair should reuse the validation diagnostic instead of adding conflicting duplicate-id wording: {}",
        stderr(&repair_output)
    );
}

#[test]
fn repair_uses_validation_wording_for_missing_dependency_references() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before missing-dependency repair setup: {init_output:?}"
    );
    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before missing-dependency repair setup: {create_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("task metadata should be editable outside SpielGantt");

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        !validate_output.status.success(),
        "validate should fail for missing dependency references: {validate_output:?}"
    );
    let repair_output = run_spielgantt_in(&project_dir, &["repair"]);
    assert!(
        !repair_output.status.success(),
        "repair should fail for missing dependency references: {repair_output:?}"
    );

    let validation_reference_lines = stderr(&validate_output)
        .lines()
        .filter(|line| line.contains("depends on missing task or event id 'calibrate-laser'"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let repair_reference_lines = stderr(&repair_output)
        .lines()
        .filter(|line| line.contains("depends on missing task or event id 'calibrate-laser'"))
        .map(str::to_string)
        .collect::<Vec<_>>();

    assert_eq!(
        validation_reference_lines.len(),
        1,
        "validate should report the invalid reference once: {}",
        stderr(&validate_output)
    );
    assert_eq!(
        repair_reference_lines,
        validation_reference_lines,
        "repair should use the same invalid-reference wording as validate: {}",
        stderr(&repair_output)
    );
}
