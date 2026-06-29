use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

#[test]
fn app_project_open_reports_a_valid_project_from_shared_validation() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should return validation status");
    let canonical_project_dir =
        std::fs::canonicalize(&project_dir).expect("project path should canonicalize");

    assert_eq!(project.selected_path(), project_dir.as_path());
    assert_eq!(
        project.project_root(),
        Some(canonical_project_dir.as_path())
    );
    assert!(project.is_valid());
    assert!(project.issues().is_empty());
    assert!(
        !project.agent_readiness().ready(),
        "application project open should surface that existing unprepared projects are not agent-ready"
    );
}

#[test]
fn app_project_open_includes_project_readme_content_and_version() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::write(
        project_dir.join("README.md"),
        "# Experiment plan\n\n- preserve package markdown\n",
    )
    .expect("project README should be writable");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include project README data");
    assert_eq!(
        project.project_readme_content(),
        "# Experiment plan\n\n- preserve package markdown\n"
    );
    let initial_version = project.project_readme_version().to_string();

    std::fs::write(
        project_dir.join("README.md"),
        "# Experiment plan\n\nexternal update\n",
    )
    .expect("project README should be externally editable");
    let reopened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should reload project README data");

    assert_eq!(
        reopened.project_readme_content(),
        "# Experiment plan\n\nexternal update\n"
    );
    assert_ne!(
        reopened.project_readme_version(),
        initial_version,
        "project README version should change when root README content changes"
    );
}

#[test]
fn app_project_open_treats_missing_project_readme_as_empty_content() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should tolerate a missing project README");

    assert_eq!(project.project_readme_content(), "");
    assert!(
        !project.project_readme_version().is_empty(),
        "missing project README should still have a stable empty-content version"
    );
}

#[test]
fn app_project_open_reports_unreadable_project_readme_path() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    std::fs::create_dir(project_dir.join("README.md"))
        .expect("directory at README path should be creatable");

    let error = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect_err("application project open should reject unreadable project README paths");

    assert!(
        error.to_string().contains("failed to read project README"),
        "error should name the project README read failure: {error}"
    );
    assert!(
        error.to_string().contains("README.md"),
        "error should include the README path: {error}"
    );
}

#[test]
fn app_agent_prepare_refreshes_scaffolding_and_returns_updated_project_readiness() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should return validation status");
    assert!(
        !opened.agent_readiness().ready(),
        "freshly initialized existing projects should still need agent scaffolding"
    );

    let prepared = spielgantt_lib::app_facade::prepare_agent_scaffolding(&project_dir)
        .expect("application agent prepare command should refresh scaffolding");

    assert!(
        prepared.project().agent_readiness().ready(),
        "application agent prepare should return a refreshed project snapshot with ready agent state"
    );
    assert!(
        project_dir.join("AGENTS.md").is_file(),
        "application agent prepare should use the shared scaffold writer"
    );
}

#[test]
fn app_project_open_includes_core_scanned_tasks_for_task_list() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "sample-prep")
        .expect("sample-prep task should be created");
    spielgantt_lib::task::create(&project_dir, "literature-review")
        .expect("literature-review task should be created");
    spielgantt_lib::task::add_dependency(&project_dir, "sample-prep", "literature-review")
        .expect("dependency should be added");
    spielgantt_lib::task::update(
        &project_dir,
        "sample-prep",
        spielgantt_lib::task::TaskUpdate {
            status: Some(spielgantt_lib::metadata::TaskStatus::Unblocked),
        },
    )
    .expect("task metadata should be updated");

    std::fs::write(
        project_dir.join("sample-prep/.spielgantt/links.json"),
        "stale links metadata should be ignored by application project open",
    )
    .expect("stale links metadata should be written");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include task scan results");
    let tasks = project.tasks();
    let task_ids = tasks.iter().map(|task| task.id()).collect::<Vec<_>>();

    assert_eq!(task_ids, vec!["literature-review", "sample-prep"]);

    let sample_prep = tasks
        .iter()
        .find(|task| task.id() == "sample-prep")
        .expect("sample-prep should be present");
    assert_eq!(sample_prep.project_relative_path(), "sample-prep");
    assert_eq!(
        sample_prep.dependencies(),
        &["literature-review".to_string()]
    );
    assert_eq!(
        tasks
            .iter()
            .find(|task| task.id() == "literature-review")
            .expect("literature-review task should be present")
            .blocks()
            .iter()
            .map(|blocked_task| (blocked_task.id(), blocked_task.kind()))
            .collect::<Vec<_>>(),
        vec![(
            "sample-prep",
            spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind::Task,
        )],
        "application project open should expose backend-computed reverse task relationships"
    );
    assert_eq!(
        sample_prep.status(),
        Some(&spielgantt_lib::metadata::TaskStatus::Unblocked)
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("sample-prep/.spielgantt/links.json"))
            .expect("stale links file should remain readable"),
        "stale links metadata should be ignored by application project open",
        "application project open should not validate or rewrite stale links metadata"
    );
}

#[test]
fn app_project_open_offers_plain_task_blocker_for_task_that_ends_at_event() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"Workflow started\",\n    \"Protocol selected\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "make chart")
        .expect("make chart task should be created");
    std::fs::write(
        project_dir.join("make chart").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"make chart\",\n  \"dependencies\": [\n    \"Workflow started\"\n  ]\n}\n",
    )
    .expect("make chart metadata should be written");

    spielgantt_lib::task::create(&project_dir, "literature-review")
        .expect("literature-review task should be created");
    std::fs::write(
        project_dir
            .join("literature-review")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"literature-review\",\n  \"ends_at\": \"Protocol selected\"\n}\n",
    )
    .expect("literature-review metadata should be written");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should expose dependency targets");
    let literature_review = project
        .tasks()
        .iter()
        .find(|task| task.id() == "literature-review")
        .expect("literature-review task should be present");

    assert!(
        literature_review
            .dependency_targets()
            .iter()
            .any(|target| target.id() == "make chart"),
        "application dependency targets should include the new plain task as a blocker option: {:?}",
        literature_review.dependency_targets()
    );
}

#[test]
fn app_project_open_exposes_ordered_events_dependency_references_and_ends_at() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("analyze-results task should be created");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be written");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include event graph data");

    assert_eq!(
        project.events(),
        &["START".to_string(), "MOT".to_string(), "BEC".to_string()],
        "application project open should preserve ordered project events"
    );
    assert_eq!(
        project
            .event_references()
            .iter()
            .map(|event| (event.id(), event.referenced_task_ids().to_vec()))
            .collect::<Vec<_>>(),
        vec![
            ("START", vec!["analyze-results".to_string()]),
            ("MOT", Vec::new()),
            ("BEC", vec!["prepare-samples".to_string()]),
        ],
        "application project open should expose Rust-provided event relationship lists"
    );
    let serialized_project =
        serde_json::to_value(&project).expect("application project should serialize to JSON");
    assert_eq!(
        serialized_project["eventReferences"],
        serde_json::json!([
            {
                "id": "START",
                "referencedTaskIds": ["analyze-results"],
                "blockerTaskIds": [],
                "blockedTaskIds": ["analyze-results"],
            },
            {
                "id": "MOT",
                "referencedTaskIds": [],
                "blockerTaskIds": [],
                "blockedTaskIds": [],
            },
            {
                "id": "BEC",
                "referencedTaskIds": ["prepare-samples"],
                "blockerTaskIds": ["prepare-samples"],
                "blockedTaskIds": [],
            },
        ]),
        "serialized application payload should include event blocker/block task IDs"
    );
    let serialized_workflow_tasks = serialized_project["workflow"]["tasks"]
        .as_array()
        .expect("application workflow tasks should serialize as an array");
    let serialized_prepare_samples = serialized_workflow_tasks
        .iter()
        .find(|task| task["id"] == "prepare-samples")
        .expect("prepare-samples workflow task should be serialized");
    assert_eq!(
        serialized_prepare_samples["effective_anchors"],
        serde_json::json!({
            "upstream": null,
            "downstream": "BEC",
            "diagnostics": [
                "task 'prepare-samples' cannot be placed on the event axis without an upstream event anchor"
            ]
        }),
        "serialized application workflow payload should pass through Rust-provided effective anchors"
    );

    let analyze_results = project
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("analyze-results task should be present");
    assert_eq!(
        analyze_results
            .dependency_references()
            .iter()
            .map(|dependency| (dependency.id(), dependency.kind()))
            .collect::<Vec<_>>(),
        vec![
            (
                "START",
                spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind::Event,
            ),
            (
                "prepare-samples",
                spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind::Task,
            ),
        ],
        "application project open should resolve task and event dependency references distinctly"
    );

    let prepare_samples = project
        .tasks()
        .iter()
        .find(|task| task.id() == "prepare-samples")
        .expect("prepare-samples task should be present");
    assert_eq!(
        prepare_samples.ends_at(),
        Some("BEC"),
        "application project open should surface optional ends_at event references"
    );
}

#[test]
fn app_shared_dependency_relationships_returns_domain_contract_shape() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("analyze-results task should be created");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be written");

    let relationships = spielgantt_lib::app_facade::dependency_relationships(&project_dir)
        .expect("application shared adapter should return dependency relationships");
    let relationship_json =
        serde_json::to_value(&relationships).expect("relationships should serialize");

    assert_eq!(relationship_json["schema_version"], 1);
    assert_eq!(
        relationship_json["tasks"][1]["blocks"],
        serde_json::json!([
            {"id": "analyze-results", "kind": "task"}
        ]),
        "shared Tauri adapter data should expose reverse task relationships"
    );
    assert_eq!(
        relationship_json["events"][1]["deletion_blockers"],
        serde_json::json!([
            {"task_id": "prepare-samples", "kind": "ends_at"}
        ]),
        "shared Tauri adapter data should expose event deletion blockers"
    );
}

#[test]
fn app_project_open_includes_the_same_workflow_domain_contract_as_cli_json() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("analyze-results task should be created");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be written");

    let cli_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        cli_output.status.success(),
        "CLI workflow query should succeed: {cli_output:?}"
    );
    let cli_workflow: serde_json::Value =
        serde_json::from_str(&stdout(&cli_output)).expect("CLI workflow should be JSON");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include workflow domain data");
    let project_json =
        serde_json::to_value(&project).expect("application project should serialize");

    assert_eq!(
        project_json["workflow"], cli_workflow,
        "Tauri/shared project open should expose the same workflow domain contract as CLI JSON"
    );
}

#[test]
fn cli_and_app_semantic_projections_report_the_same_graph_facts() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    let init_output = support::init_project(workspace.path(), "experiment-plan");
    assert!(
        init_output.status.success(),
        "init should succeed before semantic projection checks: {init_output:?}"
    );

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    for task_id in ["prepare-samples", "analyze-results", "screen-data"] {
        spielgantt_lib::task::create(&project_dir, task_id)
            .unwrap_or_else(|error| panic!("task {task_id} should be created: {error}"));
    }
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be written");
    std::fs::write(
        project_dir.join("screen-data").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"MOT\"\n}\n",
    )
    .expect("screen-data metadata should be written");

    let validate_output = run_spielgantt_in(&project_dir, &["validate", "--json"]);
    assert!(
        validate_output.status.success(),
        "fixture should be valid before agent snapshot: stdout={} stderr={}",
        stdout(&validate_output),
        support::stderr_text(&validate_output)
    );
    let snapshot_output = run_spielgantt_in(&project_dir, &["agent", "snapshot", "--json"]);
    assert!(
        snapshot_output.status.success(),
        "agent snapshot should succeed: {snapshot_output:?}"
    );
    let snapshot: serde_json::Value =
        serde_json::from_str(&stdout(&snapshot_output)).expect("snapshot should be JSON");
    let relationships_output =
        run_spielgantt_in(&project_dir, &["task", "relationships", "--json"]);
    assert!(
        relationships_output.status.success(),
        "task relationships should succeed: {relationships_output:?}"
    );
    let relationships: serde_json::Value =
        serde_json::from_str(&stdout(&relationships_output)).expect("relationships should be JSON");
    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow should succeed: {workflow_output:?}"
    );
    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("workflow should be JSON");
    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should expose semantic projections");
    let project_json =
        serde_json::to_value(&project).expect("application project should serialize");

    assert_eq!(project_json["events"], snapshot["events"]);
    assert_eq!(project_json["workflow"], workflow);

    let snapshot_analyze = snapshot["tasks"]
        .as_array()
        .expect("snapshot tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("snapshot should report analyze-results");
    let relationship_analyze = relationships["tasks"]
        .as_array()
        .expect("relationship tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("relationships should report analyze-results");
    let workflow_analyze = workflow["tasks"]
        .as_array()
        .expect("workflow tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("workflow should report analyze-results");
    let app_analyze = project_json["tasks"]
        .as_array()
        .expect("application tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("application payload should report analyze-results");

    assert_eq!(
        app_analyze["dependencyReferences"],
        snapshot_analyze["dependency_references"],
        "application payload should not rederive dependency references separately from the snapshot contract"
    );
    assert_eq!(
        relationship_analyze["blockers"],
        snapshot_analyze["dependency_references"],
        "relationship blockers and snapshot dependency references should classify the same graph references"
    );
    assert_eq!(
        workflow_analyze["dependency_references"],
        serde_json::json!([
            {
                "id": "BEC",
                "kind": "event",
                "valid": true
            }
        ]),
        "workflow JSON should expose Rust-owned dependency reference classification"
    );
    assert_eq!(
        workflow_analyze["invalid_references"],
        serde_json::json!([])
    );
    assert_eq!(
        workflow_analyze["unresolved_references"],
        serde_json::json!([])
    );
    assert!(
        workflow_analyze["effective_anchors"]["diagnostics"]
            .as_array()
            .expect("effective anchor diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic.as_str().is_some_and(|diagnostic| {
                diagnostic.contains("without a downstream event anchor")
            })),
        "workflow effective-anchor diagnostics should stay in Rust/core semantics"
    );

    let relationship_events = relationships["events"]
        .as_array()
        .expect("relationship events should be an array");
    let bec_relationship = relationship_events
        .iter()
        .find(|event| event["id"] == "BEC")
        .expect("relationships should report BEC");
    let bec_app_references = project_json["eventReferences"]
        .as_array()
        .expect("application event references should be an array")
        .iter()
        .find(|event| event["id"] == "BEC")
        .expect("application payload should report BEC references");
    assert_eq!(
        bec_relationship["deletion_blockers"],
        serde_json::json!([
            {"task_id": "analyze-results", "kind": "dependency"},
            {"task_id": "prepare-samples", "kind": "ends_at"}
        ])
    );
    assert_eq!(
        bec_app_references["blockerTaskIds"],
        serde_json::json!(["prepare-samples"]),
        "application event payload should reuse Rust relationship semantics for event blockers"
    );

    let mut visual_fields = Vec::new();
    collect_visual_payload_fields(&snapshot, "snapshot", &mut visual_fields);
    collect_visual_payload_fields(&relationships, "relationships", &mut visual_fields);
    collect_visual_payload_fields(&workflow, "workflow", &mut visual_fields);
    collect_visual_payload_fields(&project_json, "project", &mut visual_fields);
    assert!(
        visual_fields.is_empty(),
        "semantic projections must not expose frontend visual layout fields: {visual_fields:?}"
    );

    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"BEC\",\n    \"MISSING-BLOCKER\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be rewritten with an unresolved dependency");
    let invalid_workflow =
        support::run_spielgantt_json_in(&project_dir, &["task", "workflow", "--json"]);
    let invalid_workflow_analyze = invalid_workflow["tasks"]
        .as_array()
        .expect("invalid workflow tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("invalid workflow should report analyze-results");
    assert_eq!(
        invalid_workflow_analyze["unresolved_references"],
        serde_json::json!([
            {"id": "MISSING-BLOCKER", "kind": "dependency"}
        ]),
        "workflow JSON should expose Rust-owned unresolved reference diagnostics"
    );
    let invalid_project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should still expose invalid workflow semantics");
    let invalid_project_json = serde_json::to_value(&invalid_project)
        .expect("invalid application project should serialize");
    assert_eq!(
        invalid_project_json["workflow"], invalid_workflow,
        "application payload should reuse the workflow contract for unresolved references"
    );
}

#[test]
fn app_project_open_payloads_match_one_project_graph_projection() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\",\n  \"status\": \"blocked\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("analyze-results task should be created");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-samples\",\n    \"MISSING-BLOCKER\"\n  ],\n  \"status\": \"done\"\n}\n",
    )
    .expect("analyze-results metadata should be written");

    spielgantt_lib::task::create(&project_dir, "screen-data")
        .expect("screen-data task should be created");
    std::fs::write(
        project_dir.join("screen-data").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("screen-data metadata should be written");

    let graph =
        spielgantt_lib::project_graph::load(&project_dir).expect("project graph should load");
    let snapshot = spielgantt_lib::project_snapshot::from_graph(&graph);
    let relationships = spielgantt_lib::dependency_relationships::from_graph(&graph);
    let workflow = spielgantt_lib::event_axis_workflow::from_graph(&graph);

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should project graph-backed payloads");
    assert_eq!(
        project.events(),
        snapshot.events(),
        "application event list should come from the same graph projection as the task snapshot"
    );
    assert_eq!(
        project.workflow(),
        Some(&workflow),
        "application workflow diagnostics should match the graph-backed workflow projection"
    );

    for expected_task in snapshot.tasks() {
        let app_task = project
            .tasks()
            .iter()
            .find(|task| task.id() == expected_task.id())
            .expect("application payload should include every graph-backed snapshot task");
        assert_eq!(app_task.dependencies(), expected_task.dependencies());
        assert_eq!(app_task.blocks(), expected_task.blocks());
        assert_eq!(
            app_task.dependency_references(),
            expected_task.dependency_references()
        );
        assert_eq!(
            app_task.dependency_targets(),
            expected_task.dependency_targets()
        );
    }

    let project_json =
        serde_json::to_value(&project).expect("application project should serialize");
    let relationship_json =
        serde_json::to_value(&relationships).expect("relationships should serialize");
    assert_eq!(
        project_json["eventReferences"],
        serde_json::json!([
            {
                "id": relationship_json["events"][0]["id"],
                "referencedTaskIds": ["prepare-samples"],
                "blockerTaskIds": [],
                "blockedTaskIds": ["prepare-samples"],
            },
            {
                "id": relationship_json["events"][1]["id"],
                "referencedTaskIds": ["screen-data"],
                "blockerTaskIds": [],
                "blockedTaskIds": ["screen-data"],
            },
            {
                "id": relationship_json["events"][2]["id"],
                "referencedTaskIds": ["prepare-samples"],
                "blockerTaskIds": ["prepare-samples"],
                "blockedTaskIds": [],
            },
        ]),
        "application event reference payload should agree with the graph-backed relationship projection"
    );

    let workflow_json = serde_json::to_value(&workflow).expect("workflow should serialize");
    let analyze_results_workflow = workflow_json["tasks"]
        .as_array()
        .expect("workflow tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("analyze-results workflow diagnostics should be present");
    assert!(
        analyze_results_workflow["validation_diagnostics"]
            .as_array()
            .expect("workflow diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic
                .as_str()
                .is_some_and(|diagnostic| diagnostic.contains("depends on missing task or event"))),
        "workflow diagnostics should remain visible for the application task payload fixture"
    );
}

#[test]
fn app_project_open_does_not_expose_visual_event_axis_layout_fields() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"MOT\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "screen-data")
        .expect("screen-data task should be created");
    std::fs::write(
        project_dir
            .join("screen-data")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("screen-data metadata should be written");

    spielgantt_lib::task::create(&project_dir, "conflicted-run")
        .expect("conflicted-run task should be created");
    std::fs::write(
        project_dir
            .join("conflicted-run")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"conflicted-run\",\n  \"dependencies\": [\n    \"MOT\"\n  ],\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("conflicted-run metadata should be written");

    spielgantt_lib::task::create(&project_dir, "unanchored-run")
        .expect("unanchored-run task should be created");
    std::fs::write(
        project_dir
            .join("unanchored-run")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"unanchored-run\"\n}\n",
    )
    .expect("unanchored-run metadata should be written");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include workflow domain semantics");
    let project_json =
        serde_json::to_value(&project).expect("application project should serialize");

    assert!(
        project_json.get("workflow").is_some(),
        "application project open should expose backend workflow semantics"
    );
    let mut visual_fields = Vec::new();
    collect_visual_payload_fields(&project_json, "$", &mut visual_fields);
    assert!(
        visual_fields.is_empty(),
        "visual event-axis layout belongs in the frontend, not the Rust/Tauri contract: {visual_fields:?}"
    );
}

fn collect_visual_payload_fields(
    value: &serde_json::Value,
    path: &str,
    visual_fields: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                if is_visual_payload_field(key) {
                    visual_fields.push(format!("{path}.{key}"));
                }
                collect_visual_payload_fields(child, &format!("{path}.{key}"), visual_fields);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_visual_payload_fields(child, &format!("{path}[{index}]"), visual_fields);
            }
        }
        _ => {}
    }
}

fn is_visual_payload_field(key: &str) -> bool {
    matches!(
        key,
        "row"
            | "lane"
            | "grid"
            | "pixel"
            | "pixels"
            | "css"
            | "connector"
            | "connectors"
            | "labelPlacement"
            | "label_placement"
            | "visualLabelPlacement"
            | "visual_label_placement"
            | "eventAxisLayoutRows"
            | "event_axis_layout_rows"
    )
}

#[test]
fn app_project_open_exposes_backend_computed_dependency_targets() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    std::fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be written");

    spielgantt_lib::task::create(&project_dir, "analyze-results")
        .expect("analyze-results task should be created");
    std::fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be written");

    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("calibrate-laser task should be created");
    spielgantt_lib::task::create(&project_dir, "prepare-samples")
        .expect("prepare-samples task should be created");
    std::fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"analyze-results\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be written");

    spielgantt_lib::task::create(&project_dir, "screen-data")
        .expect("screen-data task should be created");
    std::fs::write(
        project_dir
            .join("screen-data")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"analyze-results\"\n  ]\n}\n",
    )
    .expect("screen-data metadata should be written");

    let project = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include dependency target choices");
    let analyze_results = project
        .tasks()
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("analyze-results task should be present");

    assert_eq!(
        analyze_results
            .dependency_targets()
            .iter()
            .map(|target| (target.id(), target.kind()))
            .collect::<Vec<_>>(),
        vec![
            (
                "calibrate-laser",
                spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind::Task,
            ),
            (
                "MOT",
                spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind::Event,
            ),
        ],
        "application open payload should expose the shared graph's valid dependency targets"
    );
}

#[test]
fn app_project_open_reports_actionable_validation_errors_for_invalid_project() {
    let workspace = tempdir().expect("workspace should be created");
    let notes_dir = workspace.path().join("notes");
    std::fs::create_dir(&notes_dir).expect("notes directory should be created");

    let project = spielgantt_lib::app_facade::open_project(&notes_dir)
        .expect("application project open should return invalid validation status");

    assert_eq!(project.selected_path(), notes_dir.as_path());
    assert_eq!(project.project_root(), None);
    assert!(!project.is_valid());
    assert!(
        project
            .issues()
            .iter()
            .any(|issue| issue.contains("missing project metadata")),
        "invalid project should report the shared validation message: {project:?}"
    );
}

#[test]
fn app_project_open_reports_missing_paths_as_invalid_project_results() {
    let workspace = tempdir().expect("workspace should be created");
    let missing_dir = workspace.path().join("moved-project");

    let project = spielgantt_lib::app_facade::open_project(&missing_dir)
        .expect("application project open should return invalid status for missing paths");

    assert_eq!(project.selected_path(), missing_dir.as_path());
    assert_eq!(project.project_root(), None);
    assert!(!project.is_valid());
    assert!(project.tasks().is_empty());
    assert!(project.events().is_empty());
    assert!(
        project
            .issues()
            .iter()
            .any(|issue| issue.contains("missing project metadata")),
        "missing project should report the shared validation message: {project:?}"
    );
}
