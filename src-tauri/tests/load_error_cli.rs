use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text};

#[test]
fn graph_load_failures_preserve_command_specific_start_path_context() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let start_path = workspace_dir.path();
    let start_path_text = start_path.display().to_string();

    let relationships_output = run_spielgantt_in(start_path, &["task", "relationships", "--json"]);
    assert!(
        !relationships_output.status.success(),
        "relationships should fail outside a SpielGantt project: {relationships_output:?}"
    );
    let relationships_error = stderr_text(&relationships_output);
    assert!(
        relationships_error.contains("no SpielGantt project found at or above"),
        "relationships should keep projection-specific error wording: {relationships_error}"
    );
    assert!(
        relationships_error.contains(&start_path_text),
        "relationships should report the selected start path: {relationships_error}"
    );

    let dependency_output = run_spielgantt_in(start_path, &["task", "depend", "analysis", "setup"]);
    assert!(
        !dependency_output.status.success(),
        "dependency mutation should fail outside a SpielGantt project: {dependency_output:?}"
    );
    let dependency_error = stderr_text(&dependency_output);
    assert!(
        dependency_error.contains("cannot add dependency from"),
        "dependency mutation should keep command-specific error wording: {dependency_error}"
    );
    assert!(
        dependency_error.contains(&start_path_text),
        "dependency mutation should report the selected start path: {dependency_error}"
    );
}
