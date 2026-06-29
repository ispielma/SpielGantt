use tempfile::tempdir;

#[test]
fn project_root_discovery_uses_canonical_paths() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let outside_dir = workspace_dir.path().join("outside");

    std::fs::create_dir(&project_dir).expect("project directory should be created");
    std::fs::create_dir(&outside_dir).expect("outside directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project initialization should succeed");

    let lexical_escape = project_dir.join("../outside");
    assert_eq!(
        spielgantt_lib::project::find_root(&lexical_escape),
        None,
        "project root discovery must not treat lexical ancestors as real ancestors"
    );
}
