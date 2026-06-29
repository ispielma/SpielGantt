import { invoke } from "@tauri-apps/api/core";
import type {
  BackendHealth,
  AdoptableTaskFolder,
  OpenProjectResult,
  ProjectAgentPrepareResult,
  ProjectActionResult,
  ProjectReadmeEdit,
  ProjectWatchSubscription,
  TaskActionResult,
  TaskDeleteMode,
  TaskEdit,
  TaskFolderAlignmentPlan,
  TaskFolderAlignmentResult,
  TaskNormalizationResult,
} from "./shell-types.ts";

export async function loadBackendHealth(): Promise<BackendHealth> {
  return invoke<BackendHealth>("spielgantt_health");
}

export async function pickProjectFolder(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open SpielGantt Project",
  });

  return typeof selected === "string" ? selected : null;
}

export async function pickProjectParentDestination(
  defaultPath?: string | null,
): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    canCreateDirectories: true,
    defaultPath: defaultPath ?? undefined,
    directory: true,
    multiple: false,
    recursive: true,
    title: "Choose Project Destination",
  });

  return typeof selected === "string" ? selected : null;
}

export async function pickTaskFolder(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Adopt Existing Task Folder",
  });

  return typeof selected === "string" ? selected : null;
}

export async function resolveDefaultProjectParent(): Promise<string | null> {
  const { documentDir, join } = await import("@tauri-apps/api/path");
  const documentsPath = await documentDir();
  return documentsPath ? join(documentsPath, "Projects") : null;
}

export async function openProject(path: string): Promise<OpenProjectResult> {
  return invoke<OpenProjectResult>("spielgantt_open_project", { path });
}

export async function onboardProject(path: string): Promise<OpenProjectResult> {
  return invoke<OpenProjectResult>("spielgantt_onboard_project", { projectPath: path });
}

export async function createProject(
  projectName: string,
  parentDestination: string,
): Promise<OpenProjectResult> {
  return invoke<OpenProjectResult>("spielgantt_create_project", {
    projectName,
    parentDestination,
  });
}

export async function prepareAgentScaffolding(
  projectRoot: string,
): Promise<ProjectAgentPrepareResult> {
  return invoke<ProjectAgentPrepareResult>("spielgantt_prepare_agent_scaffolding", {
    projectPath: projectRoot,
  });
}

export async function createTask(projectRoot: string, id: string): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_create_task", {
    projectPath: projectRoot,
    id,
  });
}

export async function insertTaskRelative(
  projectRoot: string,
  mode: "before" | "after",
  selectedTaskId: string,
  insertedTaskId: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>(
    mode === "before" ? "spielgantt_insert_task_before" : "spielgantt_insert_task_after",
    {
      projectPath: projectRoot,
      selectedTaskId,
      insertedTaskId,
    },
  );
}

export async function createEvent(projectRoot: string, id: string): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_create_event", {
    projectPath: projectRoot,
    id,
  });
}

export async function renameTask(
  projectRoot: string,
  taskId: string,
  newTaskId: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_rename_task", {
    projectPath: projectRoot,
    oldId: taskId,
    newId: newTaskId,
  });
}

export async function renameEvent(
  projectRoot: string,
  eventId: string,
  newEventId: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_rename_event", {
    projectPath: projectRoot,
    oldId: eventId,
    newId: newEventId,
  });
}

export async function deleteEvent(projectRoot: string, eventId: string): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_delete_event", {
    projectPath: projectRoot,
    eventId,
  });
}

export async function deleteTask(
  projectRoot: string,
  taskId: string,
  mode: TaskDeleteMode,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_delete_task", {
    projectPath: projectRoot,
    taskId,
    mode,
  });
}

export async function adoptTask(
  projectRoot: string,
  folderPath: string,
  id: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_adopt_task", {
    projectPath: projectRoot,
    folderPath,
    id,
  });
}

export async function listAdoptableTaskFolders(
  projectRoot: string,
): Promise<AdoptableTaskFolder[]> {
  return invoke<AdoptableTaskFolder[]>("spielgantt_list_adoptable_task_folders", {
    projectPath: projectRoot,
  });
}

export async function previewTaskNormalization(
  projectRoot: string,
): Promise<TaskNormalizationResult> {
  return invoke<TaskNormalizationResult>("spielgantt_preview_task_normalization", {
    projectPath: projectRoot,
  });
}

export async function applyTaskNormalization(
  projectRoot: string,
): Promise<TaskNormalizationResult> {
  return invoke<TaskNormalizationResult>("spielgantt_apply_task_normalization", {
    projectPath: projectRoot,
  });
}

export async function previewTaskFolderAlignment(
  projectRoot: string,
): Promise<TaskFolderAlignmentPlan> {
  return invoke<TaskFolderAlignmentPlan>("spielgantt_preview_task_folder_alignment", {
    projectPath: projectRoot,
  });
}

export async function applyTaskFolderAlignment(
  projectRoot: string,
  plan: TaskFolderAlignmentPlan,
): Promise<TaskFolderAlignmentResult> {
  return invoke<TaskFolderAlignmentResult>("spielgantt_apply_task_folder_alignment", {
    projectPath: projectRoot,
    plan,
  });
}

export async function editTask(
  projectRoot: string,
  taskId: string,
  edit: TaskEdit,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_edit_task", {
    projectPath: projectRoot,
    taskId,
    edit,
  });
}

export async function editProjectReadme(
  projectRoot: string,
  edit: ProjectReadmeEdit,
): Promise<ProjectActionResult> {
  return invoke<ProjectActionResult>("spielgantt_edit_project_readme", {
    projectPath: projectRoot,
    edit,
  });
}

export async function addTaskDependency(
  projectRoot: string,
  taskId: string,
  blockerId: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_add_task_dependency", {
    projectPath: projectRoot,
    taskId,
    blockerId,
  });
}

export async function removeTaskDependency(
  projectRoot: string,
  taskId: string,
  blockerId: string,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_remove_task_dependency", {
    projectPath: projectRoot,
    taskId,
    blockerId,
  });
}

export async function setTaskEndsAt(
  projectRoot: string,
  taskId: string,
  eventId: string | null,
  clear: boolean,
): Promise<TaskActionResult> {
  return invoke<TaskActionResult>("spielgantt_set_task_ends_at", {
    projectPath: projectRoot,
    taskId,
    eventId,
    clear,
  });
}

export async function openProjectFolder(projectRoot: string): Promise<void> {
  const { openPath } = await import("@tauri-apps/plugin-opener");
  await openPath(projectRoot);
}

export async function openTaskFolder(taskFolderPath: string): Promise<void> {
  const { openPath } = await import("@tauri-apps/plugin-opener");
  await openPath(taskFolderPath);
}

export async function setNativeWindowTitle(title: string): Promise<void> {
  if (typeof document !== "undefined") {
    document.title = title;
  }
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().setTitle(title);
  } catch {
    // Browser-only previews do not expose the Tauri window API.
  }
}

export async function subscribeProjectSessionChanges(
  projectRoot: string,
  refreshProject: () => Promise<void>,
): Promise<ProjectWatchSubscription> {
  const ownerWindow = globalThis.window;
  if (!ownerWindow) {
    return {
      status: { watching: false, message: "Project watch unavailable outside a window." },
      unsubscribe: () => {},
    };
  }

  const intervalId = ownerWindow.setInterval(() => {
    void refreshProject();
  }, 5000);

  return {
    status: {
      watching: true,
      message: `Polling project folder for changes: ${projectRoot}`,
    },
    unsubscribe: () => {
      ownerWindow.clearInterval(intervalId);
    },
  };
}
