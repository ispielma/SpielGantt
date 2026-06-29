use std::path::{Path, PathBuf};

use crate::MetadataRewrite;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageMutation {
    metadata_rewrites: Vec<MetadataRewrite>,
    cleanup_steps: Vec<StagedCleanup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageMutationReport {
    cleanup_failures: Vec<StagedCleanupFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedCleanup {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedCleanupFailure {
    path: PathBuf,
    error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataCommitError<E> {
    source: E,
    rollback_failures: Vec<MetadataRollbackFailure<E>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataRollbackFailure<E> {
    path: PathBuf,
    source: E,
}

impl PackageMutation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn metadata_rewrites(
        rewrites: impl IntoIterator<Item = MetadataRewrite>,
    ) -> PackageMutation {
        Self::new().with_metadata_rewrites(rewrites)
    }

    pub fn with_metadata_rewrites(
        mut self,
        rewrites: impl IntoIterator<Item = MetadataRewrite>,
    ) -> Self {
        self.metadata_rewrites.extend(rewrites);
        self
    }

    pub fn remove_dir_after_commit(mut self, path: impl Into<PathBuf>) -> Self {
        self.cleanup_steps.push(StagedCleanup { path: path.into() });
        self
    }

    pub fn commit(
        self,
    ) -> Result<PackageMutationReport, MetadataCommitError<AtomicJsonMetadataWriteError>> {
        self.commit_with_writer(write_json_metadata_file)
    }

    pub fn commit_with_writer<E, F>(
        self,
        mut write_file: F,
    ) -> Result<PackageMutationReport, MetadataCommitError<E>>
    where
        F: FnMut(&Path, &str) -> Result<(), E>,
    {
        self.commit_with_writer_and_cleanup(&mut write_file, |path| std::fs::remove_dir_all(path))
    }

    pub fn commit_with_writer_and_cleanup<E, F, C>(
        self,
        mut write_file: F,
        mut cleanup: C,
    ) -> Result<PackageMutationReport, MetadataCommitError<E>>
    where
        F: FnMut(&Path, &str) -> Result<(), E>,
        C: FnMut(&Path) -> std::io::Result<()>,
    {
        let mut applied_indices: Vec<usize> = Vec::new();

        for (index, rewrite) in self.metadata_rewrites.iter().enumerate() {
            if rewrite.original_contents() == rewrite.updated_contents() {
                continue;
            }

            if let Err(source) = write_file(rewrite.path(), rewrite.updated_contents()) {
                let rollback_failures = applied_indices
                    .iter()
                    .rev()
                    .copied()
                    .filter_map(|rollback_index| {
                        let applied_rewrite = &self.metadata_rewrites[rollback_index];
                        write_file(applied_rewrite.path(), applied_rewrite.original_contents())
                            .err()
                            .map(|source| MetadataRollbackFailure {
                                path: applied_rewrite.path().to_path_buf(),
                                source,
                            })
                    })
                    .collect();

                return Err(MetadataCommitError {
                    source,
                    rollback_failures,
                });
            }

            applied_indices.push(index);
        }

        let cleanup_failures = self
            .cleanup_steps
            .into_iter()
            .filter_map(|step| {
                cleanup(&step.path)
                    .err()
                    .map(|source| StagedCleanupFailure {
                        error: source.to_string(),
                        path: step.path,
                    })
            })
            .collect();

        Ok(PackageMutationReport { cleanup_failures })
    }
}

impl PackageMutationReport {
    pub fn cleanup_failures(&self) -> &[StagedCleanupFailure] {
        &self.cleanup_failures
    }

    pub fn into_cleanup_failures(self) -> Vec<StagedCleanupFailure> {
        self.cleanup_failures
    }
}

impl StagedCleanupFailure {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

impl<E> MetadataCommitError<E> {
    pub fn source(&self) -> &E {
        &self.source
    }

    pub fn rollback_failures(&self) -> &[MetadataRollbackFailure<E>] {
        &self.rollback_failures
    }

    pub fn into_source(self) -> E {
        self.source
    }

    pub fn map_source<T, F>(self, mut map: F) -> MetadataCommitError<T>
    where
        F: FnMut(E) -> T,
    {
        MetadataCommitError {
            source: map(self.source),
            rollback_failures: self
                .rollback_failures
                .into_iter()
                .map(|failure| MetadataRollbackFailure {
                    path: failure.path,
                    source: map(failure.source),
                })
                .collect(),
        }
    }
}

impl<E> MetadataRollbackFailure<E> {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> &E {
        &self.source
    }
}

impl<E: std::fmt::Display> std::fmt::Display for MetadataCommitError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.source)?;
        for rollback_failure in &self.rollback_failures {
            write!(
                formatter,
                "; rollback failed for '{}': {}",
                rollback_failure.path.display(),
                rollback_failure.source
            )?;
        }
        Ok(())
    }
}

impl<E> std::error::Error for MetadataCommitError<E> where E: std::error::Error + 'static {}

#[derive(Debug)]
pub enum AtomicJsonMetadataWriteError {
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    Replace {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
}

impl AtomicJsonMetadataWriteError {
    pub fn write(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }

    pub fn replace(
        from: impl Into<PathBuf>,
        to: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::Replace {
            from: from.into(),
            to: to.into(),
            source,
        }
    }
}

impl std::fmt::Display for AtomicJsonMetadataWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write { path, source } => {
                write!(
                    formatter,
                    "failed to write temporary metadata '{}': {source}",
                    path.display()
                )
            }
            Self::Replace { from, to, source } => {
                write!(
                    formatter,
                    "failed to replace metadata '{}' with '{}': {source}",
                    to.display(),
                    from.display()
                )
            }
        }
    }
}

impl std::error::Error for AtomicJsonMetadataWriteError {}

pub(crate) fn write_json_metadata_file(
    path: &Path,
    contents: &str,
) -> Result<(), AtomicJsonMetadataWriteError> {
    let temporary_path = path.with_extension("json.tmp");
    std::fs::write(&temporary_path, contents)
        .map_err(|source| AtomicJsonMetadataWriteError::write(temporary_path.clone(), source))?;
    std::fs::rename(&temporary_path, path).map_err(|source| {
        AtomicJsonMetadataWriteError::replace(temporary_path, path.to_path_buf(), source)
    })?;

    Ok(())
}
