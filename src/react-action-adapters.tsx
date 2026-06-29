import { createContext, useContext, useMemo, type ReactNode } from "react";

import { createProductionRememberedProjectsSettings } from "./remembered-projects.ts";
import { moveProjectFolderToTrash } from "./project-trash.ts";
import {
  applyTaskFolderAlignment,
  applyTaskNormalization,
  adoptTask,
  addTaskDependency,
  createEvent,
  createProject,
  createTask,
  deleteEvent,
  deleteTask,
  editProjectReadme,
  editTask,
  insertTaskRelative,
  listAdoptableTaskFolders,
  loadBackendHealth,
  onboardProject,
  openProject,
  pickProjectParentDestination,
  openProjectFolder,
  openTaskFolder,
  pickProjectFolder,
  pickTaskFolder,
  prepareAgentScaffolding,
  previewTaskFolderAlignment,
  previewTaskNormalization,
  removeTaskDependency,
  renameEvent,
  renameTask,
  resolveDefaultProjectParent,
  setNativeWindowTitle,
  setTaskEndsAt,
  subscribeProjectSessionChanges,
} from "./shell-tauri-adapter.ts";
import type { ShellDeps, ShellDepsInput } from "./shell-types.ts";

const ShellActionsContext = createContext<ShellDeps | null>(null);

export function createShellActions(overrides: ShellDepsInput = {}): ShellDeps {
  const defaults: ShellDeps = {
    projectLifecycle: {
      loadHealth: loadBackendHealth,
      openSelectedProject: openProject,
      onboardProjectAction: onboardProject,
      createProjectAction: createProject,
      editProjectReadmeAction: editProjectReadme,
      prepareAgentScaffoldingAction: prepareAgentScaffolding,
      previewAlignmentAction: previewTaskFolderAlignment,
      applyAlignmentAction: applyTaskFolderAlignment,
    },
    projectSession: {
      loadRememberedProjectsSettings: createProductionRememberedProjectsSettings,
    },
    taskMutation: {
      createTaskAction: createTask,
      insertTaskRelativeAction: insertTaskRelative,
      adoptTaskAction: adoptTask,
      listAdoptableTaskFoldersAction: listAdoptableTaskFolders,
      previewNormalization: previewTaskNormalization,
      applyNormalization: applyTaskNormalization,
      editTaskAction: editTask,
      addDependencyAction: addTaskDependency,
      removeDependencyAction: removeTaskDependency,
      renameTaskAction: renameTask,
      deleteTaskAction: deleteTask,
      setTaskEndsAtAction: setTaskEndsAt,
    },
    eventMutation: {
      createEventAction: createEvent,
      renameEventAction: renameEvent,
      deleteEventAction: deleteEvent,
    },
    desktop: {
      pickProjectFolder,
      pickProjectParentDestination,
      resolveDefaultProjectParent,
      pickTaskFolder,
      openProjectFolderAction: openProjectFolder,
      openTaskFolderAction: openTaskFolder,
      setWindowTitle: setNativeWindowTitle,
      trashProjectFolderAction: moveProjectFolderToTrash,
    },
    projectWatch: {
      subscribeProjectSessionChanges,
    },
  };

  return {
    projectLifecycle: { ...defaults.projectLifecycle, ...overrides.projectLifecycle },
    projectSession: { ...defaults.projectSession, ...overrides.projectSession },
    taskMutation: { ...defaults.taskMutation, ...overrides.taskMutation },
    eventMutation: { ...defaults.eventMutation, ...overrides.eventMutation },
    desktop: { ...defaults.desktop, ...overrides.desktop },
    projectWatch: { ...defaults.projectWatch, ...overrides.projectWatch },
  };
}

type ShellActionsProviderProps = {
  children: ReactNode;
  deps?: ShellDepsInput;
};

export function ShellActionsProvider({
  children,
  deps = {},
}: ShellActionsProviderProps) {
  const actions = useMemo(() => createShellActions(deps), [deps]);

  return <ShellActionsContext.Provider value={actions}>{children}</ShellActionsContext.Provider>;
}

export function useShellActions(): ShellDeps {
  const actions = useContext(ShellActionsContext);
  if (!actions) {
    throw new Error("useShellActions must be used within a ShellActionsProvider");
  }

  return actions;
}
