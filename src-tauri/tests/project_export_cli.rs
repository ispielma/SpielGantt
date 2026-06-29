use std::{collections::BTreeMap, fs, path::Path};

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

fn snapshot_files(root: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files);
    files
}

fn collect_files(root: &Path, current: &Path, files: &mut BTreeMap<String, String>) {
    let mut entries = fs::read_dir(current)
        .expect("directory should be readable")
        .map(|entry| entry.expect("directory entry should be readable"))
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if entry
            .file_type()
            .expect("file type should be readable")
            .is_dir()
        {
            collect_files(root, &path, files);
        } else {
            let relative_path = path
                .strip_prefix(root)
                .expect("file should be under root")
                .to_string_lossy()
                .into_owned();
            let contents = fs::read_to_string(&path).expect("file should be readable as utf-8");
            files.insert(relative_path, contents);
        }
    }
}

#[test]
fn export_lists_events_in_project_order_and_marks_dependency_targets_and_ends_at() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before export: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    );

    let create_prepare_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        create_prepare_output.status.success(),
        "task create should succeed for prepare-sample: {create_prepare_output:?}"
    );
    let create_analyze_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_analyze_output.status.success(),
        "task create should succeed for analyze-results: {create_analyze_output:?}"
    );
    let create_publish_output =
        run_spielgantt_in(&project_dir, &["task", "create", "publish-results"]);
    assert!(
        create_publish_output.status.success(),
        "task create should succeed for publish-results: {create_publish_output:?}"
    );

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "publish-results", "analyze-results"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before export: {depend_output:?}"
    );

    let event_depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "publish-results", "START"],
    );
    assert!(
        event_depend_output.status.success(),
        "task depend should allow event dependencies before export: {event_depend_output:?}"
    );

    let ends_at_output =
        run_spielgantt_in(&project_dir, &["task", "ends-at", "prepare-sample", "BEC"]);
    assert!(
        ends_at_output.status.success(),
        "task ends-at should succeed before export: {ends_at_output:?}"
    );

    let export_output = run_spielgantt_in(&project_dir, &["export"]);
    assert!(
        export_output.status.success(),
        "export should succeed for an event-aware project: {export_output:?}"
    );

    let exported_markdown = stdout(&export_output);
    assert!(
        exported_markdown.contains("## Events\n\n1. START\n2. MOT\n3. BEC\n\n"),
        "export should list project events in metadata order: {exported_markdown}"
    );
    assert!(
        exported_markdown.contains("- Ends at: `BEC`"),
        "export should render task ends_at metadata: {exported_markdown}"
    );
    assert!(
        exported_markdown.contains("  - `analyze-results` (task)"),
        "export should label task dependency references as tasks: {exported_markdown}"
    );
    assert!(
        exported_markdown.contains("  - `START` (event)"),
        "export should label event dependency references as events: {exported_markdown}"
    );

    let second_export_output = run_spielgantt_in(&project_dir, &["export"]);
    assert!(
        second_export_output.status.success(),
        "export should succeed deterministically on repeat runs: {second_export_output:?}"
    );
    assert_eq!(
        stdout(&second_export_output),
        exported_markdown,
        "export should produce deterministic markdown output"
    );
}

#[test]
fn export_writes_a_deterministic_markdown_snapshot_without_mutating_the_project() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before export: {init_output:?}"
    );

    let prepare_output = run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        prepare_output.status.success(),
        "task create should succeed for prepare-sample: {prepare_output:?}"
    );

    let analyze_output = run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        analyze_output.status.success(),
        "task create should succeed for analyze-results: {analyze_output:?}"
    );

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "prepare-sample"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before export: {depend_output:?}"
    );

    let update_output = run_spielgantt_in(
        &project_dir,
        &["task", "update", "analyze-results", "--status", "blocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed before export: {update_output:?}"
    );

    let before_export = snapshot_files(&project_dir);

    let export_output = run_spielgantt_in(&project_dir, &["export"]);
    assert!(
        export_output.status.success(),
        "export should succeed: {export_output:?}"
    );

    let exported_markdown = stdout(&export_output);
    assert_eq!(
        exported_markdown,
        concat!(
            "# SpielGantt Project Snapshot\n\n",
            "## Events\n\n",
            "1. start\n",
            "2. finished\n",
            "\n",
            "## Task `analyze-results`\n\n",
            "- Path: `analyze-results`\n",
            "- Status: `blocked`\n",
            "- Dependencies:\n",
            "  - `prepare-sample` (task)\n",
            "\n",
            "## Task `prepare-sample`\n\n",
            "- Path: `prepare-sample`\n",
            "- Status: -\n",
            "- Dependencies: none\n",
        ),
        "export should produce deterministic markdown for the project snapshot"
    );

    assert_eq!(
        snapshot_files(&project_dir),
        before_export,
        "export should not mutate project files"
    );
}
