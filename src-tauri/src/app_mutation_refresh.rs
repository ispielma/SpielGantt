use std::path::Path;

use crate::project_payload::{open_project, OpenProjectError, OpenProjectResult};

pub(crate) fn mutate_then_refresh<
    Mutation,
    MutationOutput,
    MutationError,
    ActionError,
    Build,
    ActionResult,
>(
    project_path: &Path,
    mutate: Mutation,
    map_mutation_error: impl FnOnce(MutationError) -> ActionError,
    map_refresh_error: impl FnOnce(OpenProjectError) -> ActionError,
    build_result: Build,
) -> Result<ActionResult, ActionError>
where
    Mutation: FnOnce() -> Result<MutationOutput, MutationError>,
    Build: FnOnce(OpenProjectResult, MutationOutput) -> ActionResult,
{
    let mutation_output = mutate().map_err(map_mutation_error)?;
    let project = open_project(project_path).map_err(map_refresh_error)?;
    Ok(build_result(project, mutation_output))
}
