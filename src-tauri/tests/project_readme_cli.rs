use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_json};

#[test]
fn project_update_readme_json_updates_root_readme_through_public_cli_contract() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("project open should provide the expected README version");

    let output = run_spielgantt_in(
        &project_dir,
        &[
            "project",
            "update-readme",
            "--content",
            "# Experiment plan\n\n- cli note\n",
            "--expected-version",
            opened.project_readme_version(),
            "--json",
        ],
    );

    assert!(
        output.status.success(),
        "project update-readme should succeed through the CLI: {output:?}"
    );
    let json = stdout_json(&output);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(
        json["project"]["projectReadmeContent"],
        "# Experiment plan\n\n- cli note\n"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("README.md"))
            .expect("project README should be readable"),
        "# Experiment plan\n\n- cli note\n"
    );
}
