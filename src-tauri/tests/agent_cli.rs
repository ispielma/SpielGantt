use serde_json::Value;
use tempfile::tempdir;

mod support;

#[test]
fn agent_runtime_json_reports_resolved_cli_path_package_context_and_version() {
    let runtime_output = support::run_spielgantt(&["agent", "runtime", "--json"]);
    assert!(
        runtime_output.status.success(),
        "agent runtime --json should succeed: {runtime_output:?}"
    );
    assert!(
        support::stderr_text(&runtime_output).is_empty(),
        "agent runtime --json should keep diagnostics out of the JSON payload"
    );

    let runtime: Value = serde_json::from_str(&support::stdout_text(&runtime_output))
        .expect("agent runtime --json stdout should be valid JSON");
    assert_eq!(runtime["schema_version"], 1);
    assert_eq!(runtime["version"], env!("CARGO_PKG_VERSION"));

    let executable_path = runtime["executable_path"]
        .as_str()
        .expect("runtime JSON should include executable_path");
    assert!(
        std::path::Path::new(executable_path).is_absolute(),
        "runtime executable_path should be absolute: {executable_path}"
    );
    assert!(
        executable_path
            .rsplit(std::path::MAIN_SEPARATOR)
            .next()
            .unwrap_or_default()
            .contains("spielgantt"),
        "runtime executable_path should point at the SpielGantt CLI binary: {executable_path}"
    );
    assert_eq!(runtime["package_context"]["platform"], std::env::consts::OS);
    assert_eq!(
        runtime["package_context"]["kind"], "standalone_executable",
        "Cargo integration tests should resolve the CLI binary as a standalone executable"
    );
    assert_eq!(runtime["package_context"]["package_path"], Value::Null);
}

#[test]
fn agent_snapshot_reports_empty_project_readiness_validation_and_paths() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before agent snapshot: {init_output:?}"
    );
    let canonical_project_dir =
        std::fs::canonicalize(&project_dir).expect("project directory should canonicalize");

    let snapshot = support::run_spielgantt_json_in(
        workspace_dir.path(),
        &["agent", "snapshot", "--json", "project"],
    );

    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(
        snapshot["project_root"],
        canonical_project_dir.display().to_string()
    );
    assert_eq!(
        snapshot["agent"],
        serde_json::json!({
            "ready": true,
            "agents_md_present": true,
            "skills_dir_present": true,
            "metadata_present": true,
            "recorded_cli_path": env!("CARGO_BIN_EXE_spielgantt")
        })
    );
    assert_eq!(
        snapshot["validation"],
        serde_json::json!({
            "schema_version": 1,
            "valid": true,
            "project_root": canonical_project_dir.display().to_string(),
            "issues": []
        })
    );
    assert_eq!(snapshot["tasks"], Value::Array(Vec::new()));
    assert_eq!(snapshot["events"], serde_json::json!(["start", "finished"]));
    assert_eq!(snapshot["dependencies"], Value::Array(Vec::new()));
    assert_eq!(snapshot.get("links"), None);
    assert_eq!(
        snapshot["paths"],
        serde_json::json!({
            "project_metadata": ".spielgantt/project.json",
            "agents_md": "AGENTS.md",
            "skills_dir": ".agents/skills",
            "agent_metadata": ".spielgantt/agent.json"
        })
    );
}

#[test]
fn agent_snapshot_reports_tasks_events_dependencies_and_ready_status() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before populated agent snapshot: {init_output:?}"
    );
    let canonical_project_dir =
        std::fs::canonicalize(&project_dir).expect("project directory should canonicalize");

    std::fs::write(project_dir.join("AGENTS.md"), "Project agent guidance")
        .expect("project AGENTS.md should be written");
    for skill_name in [
        "use-spielgantt",
        "setup-spielgantt",
        "update-spielgantt",
        "review-spielgantt",
    ] {
        let skill_path = project_dir
            .join(".agents/skills")
            .join(skill_name)
            .join("SKILL.md");
        std::fs::create_dir_all(
            skill_path
                .parent()
                .expect("skill file should have a parent"),
        )
        .expect("project agent skills directory should be created");
        std::fs::write(skill_path, "generated skill placeholder")
            .expect("project agent skill file should be written");
    }
    std::fs::write(
        project_dir.join(".spielgantt/agent.json"),
        "{\n  \"schema_version\": 1,\n  \"cli_path\": \"/Applications/SpielGantt.app/Contents/MacOS/spielgantt\"\n}\n",
    )
    .expect("project agent metadata should be written");

    for event in ["START", "BEC"] {
        let output = support::run_spielgantt_in(&project_dir, &["event", "create", event]);
        assert!(
            output.status.success(),
            "event {event} should be created before populated agent snapshot: {output:?}"
        );
    }
    for task in ["prepare-samples", "analyze-results"] {
        let output = support::create_task(&project_dir, task);
        assert!(
            output.status.success(),
            "task {task} should be created before populated agent snapshot: {output:?}"
        );
    }
    let update_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "update", "analyze-results", "--status", "unblocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed before populated agent snapshot: {update_output:?}"
    );
    let event_depend_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "START"],
    );
    assert!(
        event_depend_output.status.success(),
        "event dependency should be added before populated agent snapshot: {event_depend_output:?}"
    );
    let task_depend_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "prepare-samples"],
    );
    assert!(
        task_depend_output.status.success(),
        "task dependency should be added before populated agent snapshot: {task_depend_output:?}"
    );
    let status = support::run_spielgantt_json_in(
        workspace_dir.path(),
        &["agent", "status", "--json", "project"],
    );
    assert_eq!(status["schema_version"], 1);
    assert_eq!(
        status["agent"],
        serde_json::json!({
            "ready": true,
            "agents_md_present": true,
            "skills_dir_present": true,
            "metadata_present": true,
            "recorded_cli_path": "/Applications/SpielGantt.app/Contents/MacOS/spielgantt"
        })
    );
    assert_eq!(status["validation"]["valid"], true);

    let snapshot = support::run_spielgantt_json_in(
        workspace_dir.path(),
        &["agent", "snapshot", "--json", "project"],
    );
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(
        snapshot["project_root"],
        canonical_project_dir.display().to_string()
    );
    assert_eq!(
        snapshot["events"],
        serde_json::json!(["start", "START", "BEC", "finished"])
    );
    let tasks = support::json_array(&snapshot, "tasks");
    assert_eq!(tasks.len(), 2, "snapshot should report both tasks");
    let analyze_results = support::find_json_object_by_str(tasks, "id", "analyze-results");
    assert_eq!(
        analyze_results["path"],
        canonical_project_dir
            .join("analyze-results")
            .display()
            .to_string()
    );
    assert_eq!(analyze_results["project_relative_path"], "analyze-results");
    assert_eq!(
        analyze_results["dependencies"],
        serde_json::json!(["START", "prepare-samples"])
    );
    assert_eq!(
        analyze_results["dependency_references"],
        serde_json::json!([
            { "id": "START", "kind": "event" },
            { "id": "prepare-samples", "kind": "task" }
        ])
    );
    assert_eq!(analyze_results["ends_at"], Value::Null);
    assert_eq!(analyze_results["status"], "unblocked");
    assert_eq!(analyze_results.get("progress"), None);

    let prepare_samples = support::find_json_object_by_str(tasks, "id", "prepare-samples");
    assert_eq!(
        prepare_samples["path"],
        canonical_project_dir
            .join("prepare-samples")
            .display()
            .to_string()
    );
    assert_eq!(prepare_samples["project_relative_path"], "prepare-samples");
    assert_eq!(prepare_samples["dependencies"], serde_json::json!([]));
    assert_eq!(
        prepare_samples["dependency_references"],
        serde_json::json!([])
    );
    assert_eq!(prepare_samples["ends_at"], Value::Null);
    assert_eq!(prepare_samples["status"], Value::Null);
    assert_eq!(prepare_samples.get("progress"), None);

    let dependencies = support::json_array(&snapshot, "dependencies");
    assert_eq!(
        dependencies.len(),
        2,
        "snapshot should report both dependencies"
    );
    for expected_dependency in [
        serde_json::json!({ "task_id": "analyze-results", "id": "START", "kind": "event" }),
        serde_json::json!({ "task_id": "analyze-results", "id": "prepare-samples", "kind": "task" }),
    ] {
        assert!(
            dependencies.contains(&expected_dependency),
            "snapshot should include dependency {expected_dependency}: {dependencies:?}"
        );
    }
    assert_eq!(snapshot.get("links"), None);
}

#[test]
fn agent_status_json_reports_invalid_validation_with_success_status() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before invalid agent status JSON: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    );
    support::write_task_metadata(&project_dir.join("collision"), "START");

    let status_output = support::run_spielgantt_in(&project_dir, &["agent", "status", "--json"]);
    assert!(
        status_output.status.success(),
        "agent status --json should report invalid validation without failing: {status_output:?}"
    );
    assert!(
        support::stderr_text(&status_output).is_empty(),
        "agent status --json should carry validation issues in the JSON payload"
    );

    let status: Value = serde_json::from_str(&support::stdout_text(&status_output))
        .expect("agent status --json stdout should remain valid JSON for invalid projects");
    assert_eq!(status["validation"]["valid"], false);
    assert_eq!(
        status["validation"]["issues"],
        serde_json::json!(["task id 'START' collides with project event id 'START'"])
    );
    assert_eq!(status["agent"]["ready"], true);
}

#[test]
fn agent_status_requires_all_project_local_skill_files_for_readiness() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = support::create_unprepared_project(workspace_dir.path(), "project");

    std::fs::write(project_dir.join("AGENTS.md"), "Project agent guidance")
        .expect("project AGENTS.md should be written");
    std::fs::create_dir_all(project_dir.join(".agents/skills/use-spielgantt"))
        .expect("partial project agent skills directory should be created");
    std::fs::write(
        project_dir.join(".spielgantt/agent.json"),
        "{\n  \"schema_version\": 1,\n  \"cli_path\": \"/tmp/spielgantt\",\n  \"version\": \"1.0.0-rc.1\"\n}\n",
    )
    .expect("project agent metadata should be written");

    let status = support::run_spielgantt_json_in(
        workspace_dir.path(),
        &["agent", "status", "--json", "project"],
    );
    assert_eq!(status["agent"]["agents_md_present"], true);
    assert_eq!(status["agent"]["skills_dir_present"], true);
    assert_eq!(status["agent"]["metadata_present"], true);
    assert_eq!(status["agent"]["recorded_cli_path"], "/tmp/spielgantt");
    assert_eq!(
        status["agent"]["ready"], false,
        "readiness should require every bundled project-local skill SKILL.md"
    );
}
