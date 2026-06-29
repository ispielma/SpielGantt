use std::{
    cell::RefCell,
    fmt, fs,
    path::{Path, PathBuf},
};

use tempfile::tempdir;

use spielgantt_lib::{package_mutation::PackageMutation, MetadataRewrite};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestWriteError(&'static str);

impl fmt::Display for TestWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for TestWriteError {}

#[test]
fn package_mutation_commits_project_and_task_metadata_rewrites() {
    let workspace = tempdir().expect("temporary directory should be created");
    let project_metadata_path = workspace.path().join(".spielgantt/project.json");
    let task_metadata_path = workspace.path().join("task/.spielgantt/task.json");
    create_metadata_file(&project_metadata_path, "project-original");
    create_metadata_file(&task_metadata_path, "task-original");

    let report = PackageMutation::metadata_rewrites([
        MetadataRewrite::new(
            &project_metadata_path,
            "project-original",
            "project-updated",
        ),
        MetadataRewrite::new(&task_metadata_path, "task-original", "task-updated"),
    ])
    .commit()
    .expect("project and task metadata should commit through the default writer");

    assert!(
        report.cleanup_failures().is_empty(),
        "metadata-only commits should not report cleanup failures"
    );
    assert_eq!(
        fs::read_to_string(&project_metadata_path)
            .expect("project metadata should remain readable"),
        "project-updated"
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        "task-updated"
    );
}

#[test]
fn package_mutation_skips_no_op_rewrites_while_committing_later_metadata() {
    let workspace = tempdir().expect("temporary directory should be created");
    let no_op_metadata_path = workspace.path().join("noop/.spielgantt/task.json");
    let updated_metadata_path = workspace.path().join("updated/.spielgantt/task.json");
    create_metadata_file(&no_op_metadata_path, "unchanged");
    create_metadata_file(&updated_metadata_path, "old");

    PackageMutation::metadata_rewrites([
        MetadataRewrite::new(&no_op_metadata_path, "unchanged", "unchanged"),
        MetadataRewrite::new(&updated_metadata_path, "old", "new"),
    ])
    .commit_with_writer(|path, contents| {
        assert_ne!(
            path,
            no_op_metadata_path.as_path(),
            "no-op metadata rewrites should not be sent to the package metadata writer"
        );
        fs::write(path, contents).map_err(TestWriteError::from_io_error)
    })
    .expect("no-op rewrites should be skipped while later rewrites still commit");

    assert_eq!(
        fs::read_to_string(&no_op_metadata_path).expect("no-op metadata should remain readable"),
        "unchanged"
    );
    assert_eq!(
        fs::read_to_string(&updated_metadata_path)
            .expect("updated metadata should remain readable"),
        "new"
    );
}

#[test]
fn package_mutation_rolls_back_metadata_rewrites_when_a_later_write_fails() {
    let workspace = tempdir().expect("temporary directory should be created");
    let project_metadata_path = workspace.path().join(".spielgantt/project.json");
    let task_metadata_path = workspace.path().join("task/.spielgantt/task.json");
    create_metadata_file(&project_metadata_path, "project-original");
    create_metadata_file(&task_metadata_path, "task-original");

    let error = PackageMutation::metadata_rewrites([
        MetadataRewrite::new(
            &project_metadata_path,
            "project-original",
            "project-updated",
        ),
        MetadataRewrite::new(&task_metadata_path, "task-original", "task-updated"),
    ])
    .commit_with_writer(|path, contents| {
        if path == task_metadata_path {
            return Err(TestWriteError("write failed"));
        }
        fs::write(path, contents).map_err(TestWriteError::from_io_error)
    })
    .expect_err("later metadata write failure should reject the mutation");

    assert!(
        error.to_string().contains("write failed"),
        "commit failure should preserve the failed metadata write source: {error}"
    );
    assert!(
        error.rollback_failures().is_empty(),
        "rollback should succeed when the original project metadata path is writable"
    );
    assert_eq!(
        fs::read_to_string(&project_metadata_path)
            .expect("project metadata should remain readable after rollback"),
        "project-original",
        "successful earlier metadata rewrites should be rolled back"
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after failed commit"),
        "task-original",
        "failed later metadata rewrite should leave the original file intact"
    );
}

#[test]
fn package_mutation_reports_rollback_failures_without_losing_the_commit_failure() {
    let workspace = tempdir().expect("temporary directory should be created");
    let first_metadata_path = workspace.path().join("first/.spielgantt/task.json");
    let second_metadata_path = workspace.path().join("second/.spielgantt/task.json");
    create_metadata_file(&first_metadata_path, "first-original");
    create_metadata_file(&second_metadata_path, "second-original");

    let attempted_writes = RefCell::new(Vec::<(PathBuf, String)>::new());
    let error = PackageMutation::metadata_rewrites([
        MetadataRewrite::new(&first_metadata_path, "first-original", "first-updated"),
        MetadataRewrite::new(&second_metadata_path, "second-original", "second-updated"),
    ])
    .commit_with_writer(|path, contents| {
        attempted_writes
            .borrow_mut()
            .push((path.to_path_buf(), contents.to_string()));
        if path == second_metadata_path && contents == "second-updated" {
            return Err(TestWriteError("commit failed"));
        }
        if path == first_metadata_path && contents == "first-original" {
            return Err(TestWriteError("rollback failed"));
        }

        fs::write(path, contents).expect("test writer should update metadata files");
        Ok(())
    })
    .expect_err("second metadata write failure should reject the package mutation");

    assert_eq!(
        error.source().to_string(),
        "commit failed",
        "transaction error should preserve the original commit failure"
    );
    let rollback_failures = error.rollback_failures();
    assert_eq!(
        rollback_failures.len(),
        1,
        "transaction error should expose rollback failures for callers"
    );
    assert_eq!(rollback_failures[0].path(), first_metadata_path.as_path());
    assert_eq!(rollback_failures[0].source().to_string(), "rollback failed");
    assert_eq!(
        fs::read_to_string(&first_metadata_path)
            .expect("first metadata should remain readable after rollback failure"),
        "first-updated",
        "observable file state should reflect that rollback did not succeed"
    );
    assert!(
        attempted_writes
            .borrow()
            .iter()
            .any(|(path, contents)| path == &first_metadata_path && contents == "first-original"),
        "transaction should attempt to roll back already committed metadata"
    );
}

#[test]
fn package_mutation_reports_staged_cleanup_failures_after_metadata_commit() {
    let workspace = tempdir().expect("temporary directory should be created");
    let metadata_path = workspace.path().join("task/.spielgantt/task.json");
    let cleanup_file = workspace.path().join("task/.spielgantt-delete-staged");
    create_metadata_file(&metadata_path, "original");
    fs::write(&cleanup_file, "not a directory")
        .expect("cleanup fixture should be a file that remove_dir_all cannot remove");

    let report = PackageMutation::metadata_rewrites([MetadataRewrite::new(
        &metadata_path,
        "original",
        "updated",
    )])
    .remove_dir_after_commit(&cleanup_file)
    .commit_with_writer(|path, contents| {
        fs::write(path, contents).map_err(TestWriteError::from_io_error)
    })
    .expect("metadata commit should succeed even when staged cleanup fails");

    assert_eq!(
        fs::read_to_string(&metadata_path).expect("metadata should remain readable"),
        "updated",
        "metadata rewrite should commit before staged cleanup is reported"
    );
    assert!(
        cleanup_file.exists(),
        "failed cleanup should leave the staged path for manual recovery"
    );
    let cleanup_failures = report.cleanup_failures();
    assert_eq!(cleanup_failures.len(), 1);
    assert_eq!(cleanup_failures[0].path(), cleanup_file.as_path());
    assert!(
        !cleanup_failures[0].error().is_empty(),
        "cleanup failure should expose the underlying cleanup error"
    );
}

fn create_metadata_file(path: &Path, contents: &str) {
    fs::create_dir_all(
        path.parent()
            .expect("metadata path should have a parent directory"),
    )
    .expect("metadata directory should be created");
    fs::write(path, contents).expect("metadata file should be written");
}

impl TestWriteError {
    fn from_io_error(_: std::io::Error) -> Self {
        Self("write failed")
    }
}
