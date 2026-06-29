mod support;

fn repository_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri should live under the repository root")
        .to_path_buf()
}

#[test]
fn timeline_ready_fixture_project_validates_through_the_cli() {
    let fixture_project = repository_root().join("src-tauri/tests/fixtures/timeline-ready-project");
    assert!(
        fixture_project.is_dir(),
        "documentation tests should include the timeline-ready project fixture"
    );

    let output = support::run_spielgantt_in(&fixture_project, &["validate"]);

    assert!(
        output.status.success(),
        "timeline-ready fixture should validate through the public CLI: {output:?}\nstderr: {}",
        support::stderr_text(&output)
    );
    assert!(
        support::stdout_text(&output).contains("project is valid"),
        "validate should report success for the timeline-ready fixture: {}",
        support::stdout_text(&output)
    );
}

#[test]
fn timeline_ready_fixture_project_opens_with_timeline_ready_tasks_for_the_gui() {
    let fixture_project = repository_root().join("src-tauri/tests/fixtures/timeline-ready-project");

    let project = spielgantt_lib::app_facade::open_project(&fixture_project)
        .expect("GUI project open should load the timeline-ready fixture");

    assert!(
        project.is_valid(),
        "timeline-ready fixture should open as valid"
    );
    let tasks = project.tasks();
    let task_ids = tasks.iter().map(|task| task.id()).collect::<Vec<_>>();
    assert_eq!(
        task_ids,
        vec![
            "analyze-results",
            "collect-fluorescence",
            "literature-review",
            "prepare-samples"
        ],
        "GUI project open should expose fixture tasks in the shared read model"
    );
    assert!(
        !project.events().is_empty(),
        "timeline-ready fixture should define events for the GUI event axis"
    );
    let workflow = project
        .workflow()
        .expect("timeline-ready fixture should expose event-axis workflow semantics");
    assert!(
        !workflow.tasks().is_empty(),
        "timeline-ready fixture should expose workflow tasks for the GUI event axis"
    );
    assert!(
        workflow.validation().diagnostics().is_empty(),
        "timeline-ready fixture should not open with workflow validation diagnostics"
    );

    let analysis_task = tasks
        .iter()
        .find(|task| task.id() == "analyze-results")
        .expect("analysis task should be present");
    assert_eq!(
        analysis_task.dependencies(),
        &["Fluorescence captured".to_string()],
        "GUI project open should expose example event dependencies"
    );
}

#[test]
fn checked_in_fluorescence_timecourse_example_validates_and_opens() {
    let fixture_project = repository_root().join("examples/fluorescence-timecourse");
    assert!(
        fixture_project.is_dir(),
        "checked-in fluorescence example should be present"
    );

    let output = support::run_spielgantt_in(&fixture_project, &["validate"]);

    assert!(
        output.status.success(),
        "checked-in fluorescence example should validate through the public CLI: {output:?}\nstderr: {}",
        support::stderr_text(&output)
    );
    assert!(
        support::stdout_text(&output).contains("project is valid"),
        "validate should report success for the checked-in fluorescence example: {}",
        support::stdout_text(&output)
    );

    let project = spielgantt_lib::app_facade::open_project(&fixture_project)
        .expect("GUI project open should load the checked-in fluorescence example");
    assert!(
        project.is_valid(),
        "checked-in fluorescence example should open as valid"
    );
    assert_eq!(
        project
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        vec![
            "Chart",
            "analyze-results",
            "collect-fluorescence",
            "literature-review",
            "prepare-samples"
        ],
        "GUI project open should expose the checked-in example task buckets"
    );
    assert!(
        project
            .workflow()
            .expect("checked-in fluorescence example should expose workflow semantics")
            .validation()
            .diagnostics()
            .is_empty(),
        "checked-in fluorescence example should not open with workflow validation diagnostics"
    );
}

#[test]
fn user_guide_documents_cli_workflow_and_accessible_app_controls() {
    let guide_path = repository_root().join("docs/user-guide.md");
    let guide =
        std::fs::read_to_string(&guide_path).expect("Slice 31 should include a user-facing guide");

    for required_phrase in [
        ".spielgantt/project.json",
        ".spielgantt/task.json",
        "IDs",
        "folder normalization",
        "rename refactor",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- init",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- task create",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- task adopt",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- normalize",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- validate",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- task depend",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- task open",
        "cargo run --manifest-path \"$SPIELGANTT_REPO/src-tauri/Cargo.toml\" -- export",
        "remembered projects",
        "New Project...",
        "Open Existing Project...",
        "Task name",
        "Open project folder",
        "task bucket",
        "Select task",
        "Create Task",
        "Task event endpoint",
        "Open task folder",
        "Task event endpoint",
        "Task README",
        "Add blocker to selected task",
    ] {
        assert!(
            guide.contains(required_phrase),
            "user guide should document '{required_phrase}'"
        );
    }

    for removed_phrase in [
        "Open Project",
        "Adopt Folder",
        "Preview Normalize",
        "Apply Normalize",
    ] {
        assert!(
            !guide.contains(removed_phrase),
            "user guide should no longer document '{removed_phrase}'"
        );
    }
}

#[test]
fn readme_presents_project_usage_installation_and_release_status() {
    let repo_root = repository_root();
    let readme_path = repo_root.join("README.md");
    let readme = std::fs::read_to_string(&readme_path).expect("README should be readable");

    for required_phrase in [
        "SpielGantt is a local-first desktop Gantt tool for scientific workflows",
        "![Fluorescence timecourse example in SpielGantt](docs/assets/spielgantt-fluorescence-timecourse.png)",
        "What It Does",
        "Tracks scientific workflow tasks as ordinary folders",
        "Example Project",
        "examples/fluorescence-timecourse",
        "cargo run --manifest-path src-tauri/Cargo.toml -- validate examples/fluorescence-timecourse",
        "cargo run --manifest-path ../../src-tauri/Cargo.toml -- task workflow --json",
        "Open Existing Project...",
        "Project Format",
        ".spielgantt/project.json",
        ".spielgantt/task.json",
        "Install",
        "GitHub Releases",
        "SpielGantt-macos.zip",
        "Drag `SpielGantt.app` to `Applications`",
        "cargo install --path src-tauri --bin spielgantt",
        "Build From Source",
        "Prerequisites",
        "Node.js",
        "Rust",
        "Tauri v2",
        "Windows",
        "Linux",
        "npm run tauri -- build",
        "src-tauri/target/release/bundle",
        "Development",
        "npm run test:frontend",
        "npm run test:release",
        "npm run test:architecture",
        "Release Status",
        "tagged pushes matching `v*`",
        "contents: write",
        "More Documentation",
        "docs/user-guide.md",
    ] {
        assert!(
            readme.contains(required_phrase),
            "README should document '{required_phrase}'"
        );
    }

    for removed_phrase in [
        "This repository is pre-MVP but close to the first internal release",
        "Fresh Checkout Quickstart",
        "Development Setup",
        "Opening Task Folders And READMEs",
        "MVP Package Scope",
        "Release Packaging Checkpoint",
    ] {
        assert!(
            !readme.contains(removed_phrase),
            "README should no longer include '{removed_phrase}'"
        );
    }

    assert!(
        repo_root
            .join("docs/assets/spielgantt-fluorescence-timecourse.png")
            .is_file(),
        "README example image should exist"
    );
}

#[test]
fn contributing_links_to_release_and_architecture_workflow_guidance() {
    let contributing_path = repository_root().join("CONTRIBUTING.md");
    let contributing =
        std::fs::read_to_string(&contributing_path).expect("CONTRIBUTING.md should be readable");

    for required_phrase in [
        "AGENTS.md",
        "docs/development-guidance.md",
        "src-tauri/tests/support/mod.rs",
        "npm run test:architecture",
        "scripts/check-architecture.mjs",
        "npm run release:verify",
        "npm run release:build",
        "docs/release-candidate.md",
    ] {
        assert!(
            contributing.contains(required_phrase),
            "CONTRIBUTING.md should link to or mention '{required_phrase}'"
        );
    }
}

#[test]
fn startup_and_durable_guidance_document_the_json_only_metadata_contract() {
    let repo_root = repository_root();
    let agent_guidance =
        std::fs::read_to_string(repo_root.join("AGENTS.md")).expect("AGENTS.md should be readable");
    let durable_guidance = std::fs::read_to_string(repo_root.join("docs/development-guidance.md"))
        .expect("development guidance should be readable");

    for (label, guidance) in [
        ("agent startup guidance", agent_guidance.as_str()),
        ("durable development guidance", durable_guidance.as_str()),
    ] {
        for required_phrase in [
            "JSON is the only structured data format",
            "new `.spielgantt` files must be JSON, not YAML",
            "`project.yaml` to `project.json`",
            "`task.yaml` to `task.json`",
            "does not maintain task-link metadata",
            "disposable cache YAML to JSON",
            "Human CLI output may remain plain text",
            "structured CLI output must be JSON",
            "Do not use `serde-saphyr` for new metadata work",
        ] {
            assert!(
                guidance.contains(required_phrase),
                "{label} should document '{required_phrase}'"
            );
        }
    }
}

#[test]
fn durable_guidance_documents_react_mantine_frontend_expectations() {
    let guidance_path = repository_root().join("docs/development-guidance.md");
    let guidance =
        std::fs::read_to_string(&guidance_path).expect("development guidance should be readable");

    for required_phrase in [
        "React + Mantine",
        "Mantine AppShell",
        "accessible names",
        "npm run release:verify",
    ] {
        assert!(
            guidance.contains(required_phrase),
            "development guidance should document '{required_phrase}'"
        );
    }
}

#[test]
fn startup_and_durable_guidance_document_the_cli_domain_boundary_contract() {
    let repo_root = repository_root();
    let agent_guidance =
        std::fs::read_to_string(repo_root.join("AGENTS.md")).expect("AGENTS.md should be readable");
    let durable_guidance = std::fs::read_to_string(repo_root.join("docs/development-guidance.md"))
        .expect("development guidance should be readable");
    let cli_contract = std::fs::read_to_string(repo_root.join("docs/agent-cli-contract.md"))
        .expect("CLI contract should be readable");

    for required_phrase in [
        "SpielGantt is a CLI-first, local-first Gantt tool",
        "The Rust backend is the",
        "package-domain library for that CLI",
        "Backend package capabilities must be expressible",
        "through CLI commands",
        "The GUI is a user-friendly interface over some or all CLI-equivalent package",
        "User-interface, session, and desktop workflows that are not package operations",
        "belong to the frontend or desktop shell",
        "whole-project deletion",
        "must not introduce a Rust/core or",
        "CLI project-delete command",
        "frontend should be rewritable to use only CLI JSON calls plus local rendering",
    ] {
        assert!(
            agent_guidance.contains(required_phrase),
            "agent startup guidance should document '{required_phrase}'"
        );
    }

    for required_phrase in [
        "`AGENTS.md` is the authoritative project-level",
        "vision and boundary contract",
        "Package-semantic GUI behavior must have CLI parity through shared Rust",
        "Purely visual presentation, GUI session state, and desktop-only workflows",
        "Rust must expose domain contracts, not GUI-specific view models",
        "Frontend code must not derive dependency validity",
        "frontend should be rewritable to use only CLI JSON",
    ] {
        assert!(
            durable_guidance.contains(required_phrase),
            "durable development guidance should document '{required_phrase}'"
        );
    }

    for required_phrase in [
        "`AGENTS.md` is the authoritative project-level vision and boundary contract",
        "This document narrows that contract to the CLI and agent-facing JSON surface",
        "The Rust backend is the package-domain library for the CLI",
        "Any GUI backend call that affects package semantics must",
        "map to the same CLI-expressible behavior",
        "GUI/session and desktop-only workflows are outside this CLI contract",
        "whole-project deletion are not package mutations",
        "must not create agent CLI commands",
    ] {
        assert!(
            cli_contract.contains(required_phrase),
            "agent CLI contract should document '{required_phrase}'"
        );
    }
}
