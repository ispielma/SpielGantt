use crate::{app_facade, dependency_relationships, mutation_plan, task};

#[tauri::command]
fn spielgantt_health() -> app_facade::BackendHealth {
    app_facade::backend_health()
}

#[tauri::command]
fn spielgantt_open_project(
    path: std::path::PathBuf,
) -> Result<app_facade::OpenProjectResult, String> {
    app_facade::open_project(&path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_refresh_project(
    project_path: std::path::PathBuf,
) -> Result<app_facade::ProjectRefreshResult, String> {
    app_facade::refresh_project(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_edit_project_readme(
    project_path: std::path::PathBuf,
    edit: app_facade::ProjectReadmeEdit,
) -> Result<app_facade::ProjectActionResult, String> {
    app_facade::edit_project_readme(&project_path, edit).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_dependency_relationships(
    project_path: std::path::PathBuf,
) -> Result<dependency_relationships::DependencyRelationships, String> {
    app_facade::dependency_relationships(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_onboard_project(
    project_path: std::path::PathBuf,
) -> Result<app_facade::OpenProjectResult, String> {
    app_facade::onboard_project(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_create_project(
    project_name: String,
    parent_destination: std::path::PathBuf,
) -> Result<app_facade::OpenProjectResult, String> {
    app_facade::create_project_in_parent(&project_name, &parent_destination)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_prepare_agent_scaffolding(
    project_path: std::path::PathBuf,
) -> Result<app_facade::ProjectAgentPrepareResult, String> {
    app_facade::prepare_agent_scaffolding(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_create_task(
    project_path: std::path::PathBuf,
    id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::create_task(&project_path, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_insert_task_before(
    project_path: std::path::PathBuf,
    selected_task_id: String,
    inserted_task_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::insert_task_before(&project_path, &selected_task_id, &inserted_task_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_insert_task_after(
    project_path: std::path::PathBuf,
    selected_task_id: String,
    inserted_task_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::insert_task_after(&project_path, &selected_task_id, &inserted_task_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_create_event(
    project_path: std::path::PathBuf,
    id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::create_event(&project_path, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_delete_event(
    project_path: std::path::PathBuf,
    event_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::delete_event(&project_path, &event_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_delete_task(
    project_path: std::path::PathBuf,
    task_id: String,
    mode: task::DeleteTaskMode,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::delete_task(&project_path, &task_id, mode).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_adopt_task(
    project_path: std::path::PathBuf,
    folder_path: std::path::PathBuf,
    id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::adopt_task(&project_path, &folder_path, &id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_list_adoptable_task_folders(
    project_path: std::path::PathBuf,
) -> Result<Vec<task::AdoptableTaskFolder>, String> {
    app_facade::list_adoptable_task_folders(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_preview_task_normalization(
    project_path: std::path::PathBuf,
) -> Result<app_facade::TaskNormalizationResult, String> {
    app_facade::preview_task_normalization(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_preview_task_folder_alignment(
    project_path: std::path::PathBuf,
) -> Result<mutation_plan::TaskFolderAlignmentPlan, String> {
    app_facade::preview_task_folder_alignment(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_apply_task_folder_alignment(
    project_path: std::path::PathBuf,
    plan: mutation_plan::TaskFolderAlignmentPlan,
) -> Result<app_facade::TaskFolderAlignmentResult, String> {
    app_facade::apply_task_folder_alignment(&project_path, &plan).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_apply_task_normalization(
    project_path: std::path::PathBuf,
) -> Result<app_facade::TaskNormalizationResult, String> {
    app_facade::apply_task_normalization(&project_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_edit_task(
    project_path: std::path::PathBuf,
    task_id: String,
    edit: app_facade::TaskEdit,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::edit_task(&project_path, &task_id, edit).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_add_task_dependency(
    project_path: std::path::PathBuf,
    task_id: String,
    blocker_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::add_task_dependency(&project_path, &task_id, &blocker_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_rename_task(
    project_path: std::path::PathBuf,
    old_id: String,
    new_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::rename_task(&project_path, &old_id, &new_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_rename_event(
    project_path: std::path::PathBuf,
    old_id: String,
    new_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::rename_event(&project_path, &old_id, &new_id).map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_remove_task_dependency(
    project_path: std::path::PathBuf,
    task_id: String,
    blocker_id: String,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::remove_task_dependency(&project_path, &task_id, &blocker_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn spielgantt_set_task_ends_at(
    project_path: std::path::PathBuf,
    task_id: String,
    event_id: Option<String>,
    clear: bool,
) -> Result<app_facade::TaskActionResult, String> {
    app_facade::set_task_ends_at(&project_path, &task_id, event_id.as_deref(), clear)
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            spielgantt_health,
            spielgantt_open_project,
            spielgantt_refresh_project,
            spielgantt_edit_project_readme,
            spielgantt_dependency_relationships,
            spielgantt_onboard_project,
            spielgantt_create_project,
            spielgantt_prepare_agent_scaffolding,
            spielgantt_create_task,
            spielgantt_insert_task_before,
            spielgantt_insert_task_after,
            spielgantt_create_event,
            spielgantt_delete_event,
            spielgantt_delete_task,
            spielgantt_adopt_task,
            spielgantt_list_adoptable_task_folders,
            spielgantt_preview_task_normalization,
            spielgantt_preview_task_folder_alignment,
            spielgantt_apply_task_folder_alignment,
            spielgantt_apply_task_normalization,
            spielgantt_edit_task,
            spielgantt_add_task_dependency,
            spielgantt_rename_task,
            spielgantt_rename_event,
            spielgantt_remove_task_dependency,
            spielgantt_set_task_ends_at
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
