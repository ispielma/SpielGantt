import type { ShellDeps, ShellDepsInput } from "../../src/main.ts";
import { InMemoryRememberedProjectsSettings } from "../../src/remembered-projects.ts";
import { readyHealth } from "./frontend-fixtures.ts";

export type TestShellDeps = ShellDepsInput;
export type { ShellDeps, ShellDepsInput };

function unexpectedDependency(name: string): never {
  throw new Error(`unexpected shell dependency call: ${name}`);
}

export function defaultTestShellDeps(): ShellDeps {
  return {
    projectLifecycle: {
      loadHealth: async () => readyHealth(),
      openSelectedProject: async () => unexpectedDependency("openSelectedProject"),
      onboardProjectAction: async () => unexpectedDependency("onboardProjectAction"),
      createProjectAction: async () => unexpectedDependency("createProjectAction"),
      editProjectReadmeAction: async () => unexpectedDependency("editProjectReadmeAction"),
      prepareAgentScaffoldingAction: async () =>
        unexpectedDependency("prepareAgentScaffoldingAction"),
      previewAlignmentAction: async () => ({ operations: [], preflightIssues: [] }),
      applyAlignmentAction: async () => unexpectedDependency("applyAlignmentAction"),
    },
    projectSession: {
      loadRememberedProjectsSettings: async () => new InMemoryRememberedProjectsSettings(),
    },
    taskMutation: {
      createTaskAction: async () => unexpectedDependency("createTaskAction"),
      insertTaskRelativeAction: async () => unexpectedDependency("insertTaskRelativeAction"),
      adoptTaskAction: async () => unexpectedDependency("adoptTaskAction"),
      listAdoptableTaskFoldersAction: async () =>
        unexpectedDependency("listAdoptableTaskFoldersAction"),
      previewNormalization: async () => unexpectedDependency("previewNormalization"),
      applyNormalization: async () => unexpectedDependency("applyNormalization"),
      editTaskAction: async () => unexpectedDependency("editTaskAction"),
      addDependencyAction: async () => unexpectedDependency("addDependencyAction"),
      removeDependencyAction: async () => unexpectedDependency("removeDependencyAction"),
      renameTaskAction: async () => unexpectedDependency("renameTaskAction"),
      deleteTaskAction: async () => unexpectedDependency("deleteTaskAction"),
      setTaskEndsAtAction: async () => unexpectedDependency("setTaskEndsAtAction"),
    },
    eventMutation: {
      createEventAction: async () => unexpectedDependency("createEventAction"),
      renameEventAction: async () => unexpectedDependency("renameEventAction"),
      deleteEventAction: async () => unexpectedDependency("deleteEventAction"),
    },
    desktop: {
      pickProjectFolder: async () => null,
      pickProjectParentDestination: async () => null,
      resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt",
      pickTaskFolder: async () => null,
      openProjectFolderAction: async () => unexpectedDependency("openProjectFolderAction"),
      openTaskFolderAction: async () => unexpectedDependency("openTaskFolderAction"),
      setWindowTitle: async () => {},
      trashProjectFolderAction: async () => unexpectedDependency("trashProjectFolderAction"),
    },
    projectWatch: {
      subscribeProjectSessionChanges: async () => ({
        status: { watching: false, message: "watch unavailable" },
        unsubscribe: () => {},
      }),
    },
  };
}

export function resolveTestShellDeps(deps: TestShellDeps = {}): ShellDeps {
  const defaults = defaultTestShellDeps();
  return {
    projectLifecycle: { ...defaults.projectLifecycle, ...deps.projectLifecycle },
    projectSession: { ...defaults.projectSession, ...deps.projectSession },
    taskMutation: { ...defaults.taskMutation, ...deps.taskMutation },
    eventMutation: { ...defaults.eventMutation, ...deps.eventMutation },
    desktop: { ...defaults.desktop, ...deps.desktop },
    projectWatch: { ...defaults.projectWatch, ...deps.projectWatch },
  };
}
