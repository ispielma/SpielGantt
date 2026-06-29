import {
  forgetRememberedProject,
  type RememberedProjectRecord,
  type RememberedProjectsSettings,
} from "./remembered-projects.ts";
import {
  emptyRememberedProjectsSidebarState,
  type RememberedProjectsSidebarState,
} from "./sidebar-tree-model.ts";
import { noProjectOpen, projectStateFromResult } from "./shell-state.ts";
import type { OpenProjectResult, ProjectOpener, ProjectOpenState } from "./shell-types.ts";
import type { RememberedProjectsStateController } from "./use-remembered-projects-state.ts";

export interface TreeSelectionState {
  selectedTaskId: string | null;
  selectedEventId: string | null;
}

export interface TreeSelectionStateController {
  getState: () => TreeSelectionState;
  setState: (state: TreeSelectionState) => void;
}

export const emptyTreeSelectionState: TreeSelectionState = {
  selectedTaskId: null,
  selectedEventId: null,
};

export function resolveSelectedTaskId(
  project: OpenProjectResult,
  preferredTaskId: string | null,
): string | null {
  if (!preferredTaskId) {
    return null;
  }

  return project.tasks.some((task) => task.id === preferredTaskId) ? preferredTaskId : null;
}

export function resolveRememberedTaskId(
  project: OpenProjectResult,
  rememberedProjects: RememberedProjectRecord[],
): string | null {
  const rememberedTaskId =
    project.projectRoot
      ? rememberedProjects.find((record) => record.projectPath === project.projectRoot)
          ?.lastSelectedTaskName ?? null
      : null;

  return resolveSelectedTaskId(project, rememberedTaskId);
}

export function selectionForProject(
  project: OpenProjectResult,
  preferredTaskId: string | null,
  preferredEventId: string | null,
): TreeSelectionState {
  const selectedEventId =
    preferredEventId && project.events.includes(preferredEventId) ? preferredEventId : null;
  return {
    selectedTaskId: selectedEventId ? null : resolveSelectedTaskId(project, preferredTaskId),
    selectedEventId,
  };
}

export function taskFolderPathFromProjectState(
  projectState: ProjectOpenState,
  taskId: string,
): string | null {
  if (projectState.status === "none") {
    return null;
  }

  return projectState.project?.tasks.find((task) => task.id === taskId)?.path ?? null;
}

export function projectContentsSidebarState(
  sidebarState: RememberedProjectsSidebarState,
  project: OpenProjectResult,
): RememberedProjectsSidebarState {
  if (!project.projectRoot) {
    return sidebarState;
  }

  return {
    ...sidebarState,
    scanFailures: Object.fromEntries(
      Object.entries(sidebarState.scanFailures).filter(
        ([recordedProjectPath]) => recordedProjectPath !== project.projectRoot,
      ),
    ),
    projectContents: {
      ...sidebarState.projectContents,
      [project.projectRoot]: {
        taskNames: project.tasks.map((task) => task.id),
        eventNames: project.events,
      },
    },
  };
}

export function rememberedProjectsWithActiveProject(
  rememberedProjects: RememberedProjectRecord[],
  projectRoot: string,
  selectedTaskName: string | null,
  lastOpenedAt = new Date().toISOString(),
): RememberedProjectRecord[] {
  const nextRecord: RememberedProjectRecord = {
    projectPath: projectRoot,
    expanded: true,
    lastOpenedAt,
    lastSelectedTaskName: selectedTaskName,
  };
  const nextRecords = [...rememberedProjects];
  const existingIndex = nextRecords.findIndex(
    (record) => record.projectPath === projectRoot,
  );

  if (existingIndex === -1) {
    nextRecords.push(nextRecord);
  } else {
    nextRecords[existingIndex] = nextRecord;
  }

  return nextRecords.map((record) =>
    record.projectPath === projectRoot ? record : { ...record, expanded: false },
  );
}

export interface ProjectSessionChange {
  selectionChanged: boolean;
}

export interface ProjectSessionSnapshot {
  activeProjectRoot: string | null;
  projectState: ProjectOpenState;
  selection: TreeSelectionState;
  rememberedProjects: RememberedProjectRecord[];
  sidebarState: RememberedProjectsSidebarState;
}

export interface ProjectSessionOwnerOptions {
  initialRememberedProjects: RememberedProjectRecord[];
  rememberedProjectsSettings?: RememberedProjectsSettings | null;
  rememberedProjectsState?: RememberedProjectsStateController | null;
  treeSelectionState?: TreeSelectionStateController | null;
}

export interface ProjectSessionOwnerCommands {
  activateProject: (
    project: OpenProjectResult,
    preferredTaskId?: string | null,
    expandRememberedRecord?: boolean | null,
    preferredEventId?: string | null,
  ) => ProjectSessionChange;
  clearActiveProject: () => ProjectSessionChange;
  flushPersistence: () => Promise<void>;
  markRememberedProjectMissing: (projectPath: string, message: string) => void;
  refreshRememberedProjectContents: (
    projectPath: string,
    openSelectedProject: ProjectOpener,
    render: () => void,
  ) => Promise<void>;
  removeRememberedProject: (projectPath: string) => Promise<void>;
  selectEvent: (eventId: string | null) => ProjectSessionChange;
  selectEventIfPresent: (
    project: OpenProjectResult,
    eventId: string | null,
  ) => ProjectSessionChange;
  selectTask: (taskId: string | null) => ProjectSessionChange;
  toggleActiveProjectExpansion: (projectPath: string) => Promise<void>;
  updateSelection: (patch: Partial<TreeSelectionState>) => ProjectSessionChange;
}

export interface ProjectSessionOwner {
  readonly snapshot: ProjectSessionSnapshot;
  readonly commands: ProjectSessionOwnerCommands;
}

function cloneRememberedProjects(
  rememberedProjects: RememberedProjectRecord[],
): RememberedProjectRecord[] {
  return rememberedProjects.map((record) => ({ ...record }));
}

function cloneSidebarState(
  sidebarState: RememberedProjectsSidebarState,
): RememberedProjectsSidebarState {
  return {
    refreshInFlightProjectPaths: [...sidebarState.refreshInFlightProjectPaths],
    scanFailures: { ...sidebarState.scanFailures },
    projectContents: Object.fromEntries(
      Object.entries(sidebarState.projectContents).map(([projectPath, contents]) => [
        projectPath,
        {
          taskNames: [...contents.taskNames],
          eventNames: [...contents.eventNames],
        },
      ]),
    ),
  };
}

function selectionsEqual(
  left: TreeSelectionState,
  right: TreeSelectionState,
): boolean {
  return (
    left.selectedTaskId === right.selectedTaskId &&
    left.selectedEventId === right.selectedEventId
  );
}

export function createProjectSessionOwner({
  initialRememberedProjects,
  rememberedProjectsSettings = null,
  rememberedProjectsState = null,
  treeSelectionState = null,
}: ProjectSessionOwnerOptions): ProjectSessionOwner {
  let projectState: ProjectOpenState = noProjectOpen;
  let rememberedProjects = cloneRememberedProjects(initialRememberedProjects);
  let sidebarState: RememberedProjectsSidebarState = {
    ...emptyRememberedProjectsSidebarState,
  };
  let selectionState: TreeSelectionState = { ...emptyTreeSelectionState };
  let persistenceQueue = Promise.resolve();

  const getSelection = (): TreeSelectionState =>
    treeSelectionState?.getState() ?? selectionState;

  const setSelection = (selection: TreeSelectionState): ProjectSessionChange => {
    const previousSelection = getSelection();
    selectionState = selection;
    treeSelectionState?.setState(selection);
    return { selectionChanged: !selectionsEqual(previousSelection, selection) };
  };

  const enqueuePersistence = (persist: () => Promise<void>) => {
    persistenceQueue = persistenceQueue.then(persist, persist);
  };

  const recordProject = (
    project: OpenProjectResult,
    selectedTaskName: string | null,
    _expanded: boolean | null = null,
  ) => {
    if (!project.projectRoot) {
      return;
    }

    sidebarState = projectContentsSidebarState(sidebarState, project);
    rememberedProjects = rememberedProjectsWithActiveProject(
      rememberedProjects,
      project.projectRoot,
      selectedTaskName,
    );
    if (rememberedProjectsState) {
      rememberedProjects.forEach((record) => {
        enqueuePersistence(() => rememberedProjectsState.upsertProjectRecord(record));
      });
      return;
    }
    if (!rememberedProjectsSettings) {
      return;
    }
    rememberedProjects.forEach((record) => {
      enqueuePersistence(() => rememberedProjectsSettings.upsertProjectRecord(record));
    });
  };

  const activeProjectRoot = (): string | null =>
    projectState.status === "none" ? null : projectState.project.projectRoot;

  const clearActiveProject = (): ProjectSessionChange => {
    projectState = noProjectOpen;
    rememberedProjectsState?.setActiveProjectPath(null);
    return setSelection({ ...emptyTreeSelectionState });
  };

  const removeRememberedProject = async (projectPath: string): Promise<void> => {
    if (activeProjectRoot() === projectPath) {
      clearActiveProject();
    }

    rememberedProjects = rememberedProjects.filter(
      (record) => record.projectPath !== projectPath,
    );
    const { [projectPath]: _removedFailure, ...remainingFailures } =
      sidebarState.scanFailures;
    const { [projectPath]: _removedContents, ...remainingContents } =
      sidebarState.projectContents;
    void _removedFailure;
    void _removedContents;
    sidebarState = {
      ...sidebarState,
      scanFailures: remainingFailures,
      projectContents: remainingContents,
      refreshInFlightProjectPaths: sidebarState.refreshInFlightProjectPaths.filter(
        (recordedProjectPath) => recordedProjectPath !== projectPath,
      ),
    };

    if (rememberedProjectsState) {
      await rememberedProjectsState.removeProject(projectPath).catch(() => {});
    } else if (rememberedProjectsSettings) {
      await forgetRememberedProject(rememberedProjectsSettings, projectPath).catch(() => {});
    }
  };

  const commands: ProjectSessionOwnerCommands = {
    activateProject(
      project,
      preferredTaskId = getSelection().selectedTaskId,
      expandRememberedRecord = null,
      preferredEventId = getSelection().selectedEventId,
    ) {
      projectState = projectStateFromResult(project);
      rememberedProjectsState?.setActiveProjectPath(project.projectRoot);
      const selectionChange = setSelection(
        selectionForProject(project, preferredTaskId, preferredEventId),
      );
      recordProject(project, getSelection().selectedTaskId, expandRememberedRecord);
      return selectionChange;
    },
    clearActiveProject,
    flushPersistence: async () => {
      await persistenceQueue;
    },
    markRememberedProjectMissing(projectPath, message) {
      const { [projectPath]: _removedContents, ...remainingContents } =
        sidebarState.projectContents;
      void _removedContents;
      sidebarState = {
        ...sidebarState,
        scanFailures: {
          ...sidebarState.scanFailures,
          [projectPath]: message,
        },
        projectContents: remainingContents,
      };
    },
    async refreshRememberedProjectContents(projectPath, openSelectedProject, render) {
      await refreshRememberedProjectContents(projectPath, {
        getSidebarState: () => sidebarState,
        setSidebarState: (state) => {
          sidebarState = state;
        },
        openSelectedProject,
        render,
      });
    },
    removeRememberedProject,
    selectEvent(eventId) {
      return setSelection({
        selectedTaskId: null,
        selectedEventId: eventId || null,
      });
    },
    selectEventIfPresent(project, eventId) {
      if (!eventId || !project.events.includes(eventId)) {
        return { selectionChanged: false };
      }
      return setSelection({
        selectedTaskId: null,
        selectedEventId: eventId,
      });
    },
    selectTask(taskId) {
      const selectionChange = setSelection({
        selectedTaskId: taskId || null,
        selectedEventId: null,
      });
      if (projectState.status !== "none") {
        recordProject(projectState.project, getSelection().selectedTaskId);
      }
      return selectionChange;
    },
    async toggleActiveProjectExpansion(projectPath) {
      if (activeProjectRoot() !== projectPath || projectState.status === "none") {
        return;
      }
      recordProject(projectState.project, getSelection().selectedTaskId);
      sidebarState = {
        ...sidebarState,
        scanFailures: Object.fromEntries(
          Object.entries(sidebarState.scanFailures).filter(
            ([recordedProjectPath]) => recordedProjectPath !== projectPath,
          ),
        ),
      };
    },
    updateSelection(patch) {
      return setSelection({
        ...getSelection(),
        ...patch,
      });
    },
  };

  return {
    get snapshot() {
      return {
        activeProjectRoot: activeProjectRoot(),
        projectState,
        selection: getSelection(),
        rememberedProjects: cloneRememberedProjects(rememberedProjects),
        sidebarState: cloneSidebarState(sidebarState),
      };
    },
    commands,
  };
}

export interface RememberedProjectRefreshController {
  getSidebarState: () => RememberedProjectsSidebarState;
  setSidebarState: (state: RememberedProjectsSidebarState) => void;
  openSelectedProject: ProjectOpener;
  render: () => void;
}

export async function refreshRememberedProjectContents(
  projectPath: string,
  controller: RememberedProjectRefreshController,
): Promise<void> {
  const markRefreshStarted = () => {
    const sidebarState = controller.getSidebarState();
    controller.setSidebarState({
      ...sidebarState,
      refreshInFlightProjectPaths: Array.from(
        new Set([...sidebarState.refreshInFlightProjectPaths, projectPath]),
      ),
      scanFailures: Object.fromEntries(
        Object.entries(sidebarState.scanFailures).filter(
          ([recordedProjectPath]) => recordedProjectPath !== projectPath,
        ),
      ),
    });
  };

  const markRefreshFailed = (error: unknown) => {
    const sidebarState = controller.getSidebarState();
    const { [projectPath]: _removedContents, ...remainingContents } =
      sidebarState.projectContents;
    void _removedContents;
    controller.setSidebarState({
      ...sidebarState,
      scanFailures: {
        ...sidebarState.scanFailures,
        [projectPath]: error instanceof Error ? error.message : String(error),
      },
      projectContents: remainingContents,
    });
  };

  const markRefreshFinished = () => {
    const sidebarState = controller.getSidebarState();
    controller.setSidebarState({
      ...sidebarState,
      refreshInFlightProjectPaths: sidebarState.refreshInFlightProjectPaths.filter(
        (recordedProjectPath) => recordedProjectPath !== projectPath,
      ),
    });
  };

  markRefreshStarted();
  controller.render();

  try {
    const refreshedProject = await controller.openSelectedProject(projectPath);
    if (!refreshedProject.valid || !refreshedProject.projectRoot) {
      throw new Error(
        refreshedProject.issues[0] ?? `invalid project path '${projectPath}'`,
      );
    }
    controller.setSidebarState(
      projectContentsSidebarState(controller.getSidebarState(), refreshedProject),
    );
  } catch (error) {
    markRefreshFailed(error);
  } finally {
    markRefreshFinished();
    controller.render();
  }
}
