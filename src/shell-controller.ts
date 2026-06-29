import type { ComponentProps } from "react";

import {
  createProductionRememberedProjectsSettings,
  type RememberedProjectRecord,
} from "./remembered-projects.ts";
import { createShellDialogOwner } from "./shell-dialog-owner.ts";
import {
  type RememberedProjectsSectionProps,
  type ProjectContextMenuTarget,
  type SidebarNavigatorIntent,
  type SidebarNavigatorIntentContext,
  type TaskMenuSource,
} from "./sidebar-tree.tsx";
import { ShellOverlays } from "./shell-overlays.tsx";
import { moveProjectFolderToTrash } from "./project-trash.ts";
import {
  ProjectWorkspace,
} from "./work-area-panel.tsx";
import {
  buildWorkspaceSelectionModel,
} from "./workspace-selection-model.ts";
import {
  applyTaskFolderAlignment,
  applyTaskNormalization,
  adoptTask,
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
  openProject,
  onboardProject,
  openProjectFolder,
  openTaskFolder,
  pickProjectFolder,
  pickProjectParentDestination,
  pickTaskFolder,
  previewTaskFolderAlignment,
  previewTaskNormalization,
  removeTaskDependency,
  renameEvent,
  renameTask,
  resolveDefaultProjectParent,
  setNativeWindowTitle,
  setTaskEndsAt,
  addTaskDependency,
  subscribeProjectSessionChanges,
} from "./shell-tauri-adapter.ts";
import {
  openProjectActionsFallbackMenu,
  popupNativeProjectActionMenuForProject,
} from "./project-action-menus.ts";
import {
  projectActionEntriesForProject,
  type ProjectActionCommand,
} from "./project-actions.ts";
import { ProjectOpenLifecycle } from "./project-open-lifecycle.ts";
import { basenameFromPath } from "./project-tree-dom.ts";
import {
  createEventCreationWorkflow,
  createEventDeleteWorkflow,
  createEventRenameWorkflow,
} from "./event-overlay-workflows.ts";
import { createShellOverlayActionHandler } from "./shell-overlay-actions.ts";
import { OverlayInitialFocusController } from "./overlay-initial-focus.ts";
import { ShellMenuStateController } from "./shell-menu-state.ts";
import {
  eventReferencingTaskIdsFromProject,
  idleOperation,
  noProjectOpen,
} from "./shell-state.ts";
import {
  createProjectSessionOwner,
  emptyTreeSelectionState,
  taskFolderPathFromProjectState,
  type TreeSelectionState,
  type TreeSelectionStateController,
} from "./shell-project-session.ts";
import type { OpenProjectResult, OperationState, ProjectFolderPicker, ProjectOpenState, ProjectOpener, ShellDepsInput } from "./shell-types.ts";
import { createTaskFromFolderWorkflow, type TaskFromFolderWorkflowController } from "./task-from-folder-workflow.ts";
import {
  createTaskCreationWorkflow,
  createTaskDeleteWorkflow,
  createTaskRenameWorkflow,
} from "./task-overlay-workflows.ts";
import {
  createTaskInspectorWorkflow,
  type TaskInspectorWorkflow,
} from "./task-inspector-workflow.ts";
import type { RememberedProjectsStateController } from "./use-remembered-projects-state.ts";

export type * from "./shell-types.ts";

interface ShellVisualAdapters {
  navigationHost: HTMLElement;
  workAreaHost: HTMLElement;
  overlayHost: HTMLElement;
  renderNavigation?: (props: RememberedProjectsSectionProps) => void;
  renderWorkArea?: (props: ComponentProps<typeof ProjectWorkspace>) => void;
  renderOverlays?: (props: ComponentProps<typeof ShellOverlays>) => void;
  treeSelectionState?: TreeSelectionStateController;
}
export async function openProjectThroughPicker(
  pickFolder: ProjectFolderPicker,
  openSelectedProject: ProjectOpener = openProject,
): Promise<ProjectOpenState> {
  const selectedPath = await pickFolder();
  if (!selectedPath) {
    return noProjectOpen;
  }

  const project = await openSelectedProject(selectedPath);
  return project.valid
    ? { status: "valid", project }
    : { status: "invalid", project };
}

export async function mountShell(
  root: HTMLElement,
  deps: ShellDepsInput = {},
  visualAdapters: ShellVisualAdapters | null = null,
  rememberedProjectsState: RememberedProjectsStateController | null = null,
) {
  if (
    !visualAdapters?.renderNavigation ||
    !visualAdapters.renderWorkArea ||
    !visualAdapters.renderOverlays
  ) {
    throw new Error("mountShell requires React layout hosts");
  }

  const {
    projectLifecycle: {
      loadHealth = loadBackendHealth,
      openSelectedProject = openProject,
      onboardProjectAction = onboardProject,
      createProjectAction = createProject,
      editProjectReadmeAction = editProjectReadme,
      previewAlignmentAction = previewTaskFolderAlignment,
      applyAlignmentAction = applyTaskFolderAlignment,
    } = {},
    projectSession: {
      loadRememberedProjectsSettings = createProductionRememberedProjectsSettings,
    } = {},
    taskMutation: {
      createTaskAction = createTask,
      insertTaskRelativeAction = insertTaskRelative,
      adoptTaskAction = adoptTask,
      listAdoptableTaskFoldersAction = listAdoptableTaskFolders,
      previewNormalization: _previewNormalization = previewTaskNormalization,
      applyNormalization: _applyNormalization = applyTaskNormalization,
      editTaskAction = editTask,
      addDependencyAction = addTaskDependency,
      removeDependencyAction = removeTaskDependency,
      renameTaskAction = renameTask,
      deleteTaskAction = deleteTask,
      setTaskEndsAtAction = setTaskEndsAt,
    } = {},
    eventMutation: {
      createEventAction = createEvent,
      renameEventAction = renameEvent,
      deleteEventAction = deleteEvent,
    } = {},
    desktop: {
      pickProjectFolder: pickFolder = pickProjectFolder,
      pickProjectParentDestination: pickProjectParentDestinationAction =
        pickProjectParentDestination,
      resolveDefaultProjectParent: resolveDefaultProjectParentAction =
        resolveDefaultProjectParent,
      pickTaskFolder: _pickAdoptFolder = pickTaskFolder,
      openProjectFolderAction = openProjectFolder,
      openTaskFolderAction = openTaskFolder,
      setWindowTitle = setNativeWindowTitle,
      trashProjectFolderAction = moveProjectFolderToTrash,
    } = {},
    projectWatch: {
      subscribeProjectSessionChanges: subscribeProjectChanges =
        subscribeProjectSessionChanges,
    } = {},
  } = deps;
  let operationState: OperationState = idleOperation;
  const menuState = new ShellMenuStateController();
  let taskInspectorWorkflow!: TaskInspectorWorkflow;
  let taskFromFolderWorkflow: TaskFromFolderWorkflowController | null = null;
  let projectOpenLifecycle!: ProjectOpenLifecycle;
  const dialogOwner = createShellDialogOwner({
    closeTaskFromFolderDialog: () => {
      taskFromFolderWorkflow?.close();
    },
    closeAlignmentDialog: () => {
      if (projectOpenLifecycle) {
        projectOpenLifecycle.closeAlignmentDialog();
      }
    },
  });
  const overlayInitialFocus = new OverlayInitialFocusController(root);
  const rememberedProjectsSettings = rememberedProjectsState
    ? null
    : await loadRememberedProjectsSettings();
  const initialRememberedProjects: RememberedProjectRecord[] = rememberedProjectsState
    ? rememberedProjectsState.rememberedProjects
    : rememberedProjectsSettings
      ? await rememberedProjectsSettings.listProjects()
      : [];
  const projectSession = createProjectSessionOwner({
    initialRememberedProjects,
    rememberedProjectsSettings,
    rememberedProjectsState,
    treeSelectionState: visualAdapters?.treeSelectionState,
  });

  const currentProjectRoot = (): string | null =>
    projectSession.snapshot.activeProjectRoot;

  const getTreeSelectionState = () => projectSession.snapshot.selection;

  const resetTaskControlsForSelectionChange = (selectionChanged: boolean) => {
    taskInspectorWorkflow.resetControlsForSelectionChange(selectionChanged);
  };

  const updateTreeSelectionState = (patch: Partial<TreeSelectionState>) => {
    resetTaskControlsForSelectionChange(
      projectSession.commands.updateSelection(patch).selectionChanged,
    );
  };

  const refreshProjectState = (
    project: OpenProjectResult,
    preferredTaskId: string | null = getTreeSelectionState().selectedTaskId,
    expandRememberedRecord = null as boolean | null,
    preferredEventId: string | null = getTreeSelectionState().selectedEventId,
  ) => {
    resetTaskControlsForSelectionChange(
      projectSession.commands.activateProject(
        project,
        preferredTaskId,
        expandRememberedRecord,
        preferredEventId,
      ).selectionChanged,
    );
  };

  const setActionError = (error: unknown) => {
    operationState = {
      status: "error",
      message: error instanceof Error ? error.message : String(error),
    };
  };

  const closeContextMenus = () => {
    menuState.closeContextMenus();
  };
  const closeProjectMenus = () => {
    menuState.closeProjectMenus();
  };

  const closeMutationDialogs = () => {
    dialogOwner.closeOverlayDialogs();
    closeProjectMenus();
  };
  const openTaskDialog = (
    projectPath: string,
    mode: Parameters<typeof dialogOwner.openTaskDialog>[1] = "create",
    selectedTaskId = "",
  ) => {
    dialogOwner.openTaskDialog(projectPath, mode, selectedTaskId);
  };
  const openCreateTaskDialog = (projectPath: string) => {
    openTaskDialog(projectPath);
    render();
  };
  const openCreateEventDialog = (projectPath: string, assignToTaskId?: string) => {
    dialogOwner.openCreateEventDialog(projectPath, assignToTaskId);
  };

  const eventReferencingTaskIds = (eventId: string): string[] => {
    const { projectState } = projectSession.snapshot;
    if (projectState.status === "none") {
      return [];
    }

    return eventReferencingTaskIdsFromProject(projectState.project, eventId);
  };

  taskInspectorWorkflow = createTaskInspectorWorkflow({
    currentProjectRoot,
    refreshProjectState,
    openCreateEndEvent: openCreateEventDialog,
    editTaskAction,
    addDependencyAction,
    removeDependencyAction,
    setTaskEndsAtAction,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });

  projectOpenLifecycle = new ProjectOpenLifecycle(
    {
      openSelectedProject,
      previewAlignmentAction,
      applyAlignmentAction,
      subscribeProjectSessionChanges: subscribeProjectChanges,
      setWindowTitle,
    },
    {
      currentProjectRoot,
      rememberedProjects: () => projectSession.snapshot.rememberedProjects,
      activateProject: refreshProjectState,
      selectEventIfPresent: (project, eventId) => {
        resetTaskControlsForSelectionChange(
          projectSession.commands.selectEventIfPresent(project, eventId).selectionChanged,
        );
      },
      clearActiveProject: () => {
        resetTaskControlsForSelectionChange(
          projectSession.commands.clearActiveProject().selectionChanged,
        );
      },
      setProjectsMenuOpen: (open) => {
        menuState.setProjectsMenuOpen(open);
      },
      setActionError,
      setOperationIdle: () => {
        operationState = idleOperation;
      },
      render: () => render(),
    },
  );

  const openProjectWithAlignment = async (
    projectPath: string,
    preferredTaskId: string | null = null,
    preferredEventId: string | null = null,
    prepareProject: ProjectOpener | null = null,
  ) => {
    await projectOpenLifecycle.openProjectWithAlignment({
      projectPath,
      preferredTaskId,
      preferredEventId,
      prepareProject,
    });
  };

  const confirmAlignment = async () => {
    await projectOpenLifecycle.confirmAlignment();
  };

  const clearActiveProjectIfMatching = async (projectPath: string) => {
    await projectOpenLifecycle.clearActiveProjectIfMatching(projectPath);
  };

  const refreshOpenProject = async () => {
    await projectOpenLifecycle.refreshOpenProject();
  };

  const startProjectWatch = async (projectRoot: string | null) => {
    await projectOpenLifecycle.startProjectWatch(projectRoot);
  };

  const updateWindowTitleForProject = async (projectRoot: string | null) => {
    await projectOpenLifecycle.updateWindowTitleForProject(projectRoot);
  };

  const confirmDeleteProject = async () => {
    const deleteProjectDialogState = dialogOwner.getDeleteProjectDialogState();
    if (
      !deleteProjectDialogState.open ||
      deleteProjectDialogState.confirmationText !== deleteProjectDialogState.confirmationPhrase
    ) {
      return;
    }

    const projectPath = deleteProjectDialogState.projectPath;
    dialogOwner.setDeleteProjectDialogState({
      ...deleteProjectDialogState,
      submitting: true,
      errorMessage: null,
    });
    render();

    let result: Awaited<ReturnType<typeof trashProjectFolderAction>>;
    try {
      result = await trashProjectFolderAction(projectPath);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      dialogOwner.setDeleteProjectDialogState({
        ...dialogOwner.getDeleteProjectDialogState(),
        submitting: false,
        errorMessage: message,
      });
      setActionError(new Error(message));
      render();
      return;
    }

    if (result.status !== "moved-to-trash") {
      dialogOwner.setDeleteProjectDialogState({
        ...dialogOwner.getDeleteProjectDialogState(),
        submitting: false,
        errorMessage: result.message,
      });
      setActionError(new Error(result.message));
      render();
      return;
    }

    dialogOwner.closeDeleteProjectDialog();
    await clearActiveProjectIfMatching(projectPath);
    await removeRememberedProjectReference(projectPath);
    operationState = idleOperation;
    render();
  };

  const refreshRememberedProjectTasks = async (projectPath: string) => {
    await projectSession.commands.refreshRememberedProjectContents(
      projectPath,
      openSelectedProject,
      render,
    );
  };

  taskFromFolderWorkflow = createTaskFromFolderWorkflow({ listAdoptableTaskFoldersAction, adoptTaskAction, currentProjectRoot, refreshProjectState, refreshRememberedProjectTasks, closeMutationDialogs, setActionError, setOperationIdle: () => { operationState = idleOperation; }, render: () => render() });
  const taskCreationWorkflow = createTaskCreationWorkflow({
    getState: dialogOwner.getTaskDialogState,
    setState: dialogOwner.setTaskDialogState,
    close: dialogOwner.closeTaskDialog,
    closeTaskAndEventMenus: () => {
      menuState.closeTaskAndEventMenus();
    },
    currentProjectRoot,
    createTaskAction,
    insertTaskRelativeAction,
    refreshProjectState,
    refreshRememberedProjectTasks,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });
  const taskRenameWorkflow = createTaskRenameWorkflow({
    getState: dialogOwner.getRenameTaskDialogState,
    setState: dialogOwner.setRenameTaskDialogState,
    close: dialogOwner.closeRenameTaskDialog,
    closeTaskAndEventMenus: () => {
      menuState.closeTaskAndEventMenus();
    },
    currentProjectRoot,
    renameTaskAction,
    refreshProjectState,
    refreshRememberedProjectTasks,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });
  const taskDeleteWorkflow = createTaskDeleteWorkflow({
    getState: dialogOwner.getDeleteTaskDialogState,
    setState: dialogOwner.setDeleteTaskDialogState,
    close: dialogOwner.closeDeleteTaskDialog,
    getTreeSelectionState,
    updateTreeSelectionState,
    currentProjectRoot,
    deleteTaskAction,
    refreshProjectState,
    refreshRememberedProjectTasks,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });
  const eventCreationWorkflow = createEventCreationWorkflow({
    getState: dialogOwner.getEventDialogState,
    setState: dialogOwner.setEventDialogState,
    close: dialogOwner.closeEventDialog,
    createEventAction,
    setTaskEndsAtAction,
    refreshProjectState,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });
  const eventRenameWorkflow = createEventRenameWorkflow({
    getState: dialogOwner.getRenameEventDialogState,
    setState: dialogOwner.setRenameEventDialogState,
    close: dialogOwner.closeRenameEventDialog,
    currentProjectRoot,
    renameEventAction,
    refreshProjectState,
    refreshRememberedProjectTasks,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });
  const eventDeleteWorkflow = createEventDeleteWorkflow({
    getState: dialogOwner.getDeleteEventDialogState,
    setState: dialogOwner.setDeleteEventDialogState,
    close: dialogOwner.closeDeleteEventDialog,
    getTreeSelectionState,
    updateTreeSelectionState,
    currentProjectRoot,
    deleteEventAction,
    refreshProjectState,
    refreshRememberedProjectTasks,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    render: () => render(),
  });

  const toggleRememberedProjectExpansion = async (projectPath: string) => {
    await projectSession.commands.toggleActiveProjectExpansion(projectPath);
    render();
  };

  const selectTaskFromTimeline = (taskId: string) => {
    menuState.closeTaskAndEventMenus();
    resetTaskControlsForSelectionChange(
      projectSession.commands.selectTask(taskId).selectionChanged,
    );
    render();
  };

  const selectEventFromTimeline = (eventId: string) => {
    menuState.closeTaskAndEventMenus();
    resetTaskControlsForSelectionChange(
      projectSession.commands.selectEvent(eventId).selectionChanged,
    );
    operationState = idleOperation;
    render();
  };

  const activateProjectFromTree = async (projectPath: string) => {
    menuState.closeContextMenus();
    if (!projectPath) {
      return;
    }

    if (currentProjectRoot() === projectPath) {
      if (
        getTreeSelectionState().selectedTaskId !== null ||
        getTreeSelectionState().selectedEventId !== null
      ) {
        resetTaskControlsForSelectionChange(
          projectSession.commands.updateSelection({ ...emptyTreeSelectionState })
            .selectionChanged,
        );
        render();
        return;
      }
      await toggleRememberedProjectExpansion(projectPath);
      return;
    }

    const currentRecord = projectSession.snapshot.rememberedProjects.find(
      (record) => record.projectPath === projectPath,
    );
    await openAndSelectProject(projectPath, currentRecord?.lastSelectedTaskName ?? null);
  };

  const selectTaskFromTree = async (projectPath: string, taskId: string) => {
    menuState.closeContextMenus();
    if (!projectPath || !taskId) {
      return;
    }

    updateTreeSelectionState({ selectedEventId: null });
    if (currentProjectRoot() === projectPath) {
      resetTaskControlsForSelectionChange(
        projectSession.commands.selectTask(taskId).selectionChanged,
      );
      render();
      return;
    }

    await openAndSelectProject(projectPath, taskId);
  };

  const selectEventFromTree = async (projectPath: string, eventId: string) => {
    menuState.closeContextMenus();
    if (!projectPath || !eventId) {
      return;
    }

    if (currentProjectRoot() === projectPath) {
      updateTreeSelectionState({
        selectedTaskId: null,
        selectedEventId: eventId,
      });
      operationState = idleOperation;
      render();
      return;
    }

    const currentRecord = projectSession.snapshot.rememberedProjects.find(
      (record) => record.projectPath === projectPath,
    );
    await openAndSelectProject(projectPath, currentRecord?.lastSelectedTaskName ?? null, eventId);
  };

  const toggleProjectsMenu = () => {
    const opened = menuState.toggleProjectsMenu();
    if (opened) {
      dialogOwner.closeDialogsForProjectsMenuOpen();
    }
    render();
  };
  const closeProjectsMenu = () => {
    if (menuState.closeProjectsMenu()) {
      render();
    }
  };
  const closeTaskCreateMenu = () => {
    if (menuState.closeTaskCreateMenu()) {
      render();
    }
  };
  const toggleTaskCreateMenu = (projectPath: string) => {
    menuState.toggleTaskCreateMenu(projectPath);
    render();
  };

  const openNewProjectDialog = () => {
    dialogOwner.openNewProjectDialog();
    closeProjectMenus();
    render();
    void resolveDefaultProjectParentAction()
      .then((parentDestination) => {
        const newProjectDialogState = dialogOwner.getNewProjectDialogState();
        if (!newProjectDialogState.open) {
          return;
        }
        if (!parentDestination) {
          dialogOwner.setNewProjectDialogState({
            ...newProjectDialogState,
            errorMessage:
              "Could not determine the default project destination. Choose a parent destination before creating a project.",
          });
          render();
          return;
        }
        dialogOwner.setNewProjectDialogState({
          ...newProjectDialogState,
          parentDestination: newProjectDialogState.parentDestination ?? parentDestination,
        });
        render();
      })
      .catch((error) => {
        const newProjectDialogState = dialogOwner.getNewProjectDialogState();
        if (!newProjectDialogState.open) {
          return;
        }
        dialogOwner.setNewProjectDialogState({
          ...newProjectDialogState,
          errorMessage: `Could not determine the default project destination: ${
            error instanceof Error ? error.message : String(error)
          }`,
        });
        render();
      });
  };

  const openExistingProjectFromMenu = async () => {
    dialogOwner.closeOverlayDialogs();
    closeContextMenus();
    const selectedPath = await pickFolder();
    menuState.setProjectsMenuOpen(false);
    if (!selectedPath) {
      render();
      return;
    }

    await openProjectWithAlignment(selectedPath, null, null, onboardProjectAction);
  };

  const openTaskContextMenu = (
    projectPath: string,
    taskId: string,
    position: { x: number; y: number },
    source: TaskMenuSource = "sidebar",
  ) => {
    if (!projectPath || !taskId) {
      return;
    }
    if (currentProjectRoot() === projectPath) {
      updateTreeSelectionState({
        selectedTaskId: taskId,
        selectedEventId: null,
      });
    }
    menuState.openTaskContextMenu(projectPath, taskId, position, source);
    dialogOwner.closeDialogsForTaskContextMenu();
    render();
  };
  const closeTaskContextMenu = () => {
    if (menuState.closeTaskContextMenu()) {
      render();
    }
  };
  const closeEventContextMenu = () => {
    if (menuState.closeEventContextMenu()) {
      render();
    }
  };
  const closeProjectActionMenu = () => {
    if (menuState.closeProjectActionMenu()) {
      render();
    }
  };

  const openEventContextMenu = (projectPath: string, eventId: string) => {
    if (!projectPath || !eventId) {
      return;
    }
    if (currentProjectRoot() === projectPath) {
      updateTreeSelectionState({
        selectedTaskId: null,
        selectedEventId: eventId,
      });
    }
    menuState.openEventContextMenu(projectPath, eventId);
    dialogOwner.closeDialogsForEventContextMenu();
    render();
  };

  const openRenameTaskDialog = (projectPath: string, taskId: string) => {
    menuState.closeTaskContextMenu();
    dialogOwner.openRenameTaskDialog(projectPath, taskId);
    render();
  };

  const openRenameEventDialogFor = (projectPath: string, eventId: string) => {
    resetTaskControlsForSelectionChange(
      projectSession.commands.selectEvent(eventId).selectionChanged,
    );
    menuState.closeTaskAndEventMenus();
    dialogOwner.openRenameEventDialog(projectPath, eventId, eventReferencingTaskIds(eventId));
    render();
  };

  const openDeleteEventDialogFor = (projectPath: string, eventId: string) => {
    menuState.closeTaskAndEventMenus();
    dialogOwner.openDeleteEventDialog(projectPath, eventId, eventReferencingTaskIds(eventId));
    render();
  };

  const openDeleteTaskDialogFor = (projectPath: string, taskId: string) => {
    menuState.closeTaskAndEventMenus();
    dialogOwner.openDeleteTaskDialog(projectPath, taskId);
    render();
  };
  const handleTaskContextAction = async (
    projectPath: string,
    taskId: string,
    action: "open-folder" | "add-before" | "add-after" | "rename" | "delete",
  ) => {
    if (!projectPath || !taskId) {
      return;
    }
    if (action === "rename") {
      openRenameTaskDialog(projectPath, taskId);
      return;
    }
    if (action === "add-before" || action === "add-after") {
      openTaskDialog(projectPath, action, taskId);
      render();
      return;
    }
    if (action === "delete") {
      openDeleteTaskDialogFor(projectPath, taskId);
      return;
    }

    menuState.closeTaskContextMenu();
    try {
      const taskFolderPath = taskFolderPathFromProjectState(
        projectSession.snapshot.projectState,
        taskId,
      );
      if (!taskFolderPath) {
        throw new Error(`Task folder unavailable for '${taskId}'.`);
      }
      await openTaskFolderAction(taskFolderPath);
      operationState = idleOperation;
    } catch (error) {
      setActionError(error);
    }
    render();
  };

  const handleEventContextAction = async (
    projectPath: string,
    eventId: string,
    action: "rename" | "delete",
  ) => {
    if (action === "rename") {
      openRenameEventDialogFor(projectPath, eventId);
      return;
    }
    openDeleteEventDialogFor(projectPath, eventId);
  };

  const handleProjectReadmeEdit = async (
    edit: Parameters<typeof editProjectReadmeAction>[1],
  ) => {
    const projectRoot = currentProjectRoot();
    if (!projectRoot) {
      return;
    }

    try {
      const result = await editProjectReadmeAction(projectRoot, edit);
      refreshProjectState(result.project, null, null, null);
      operationState = idleOperation;
    } catch (error) {
      setActionError(error);
      render();
      throw error;
    }
    render();
  };

  const handleOverlayAction = createShellOverlayActionHandler({
    getNewProjectDialogState: dialogOwner.getNewProjectDialogState,
    setNewProjectDialogState: dialogOwner.setNewProjectDialogState,
    getDeleteProjectDialogState: dialogOwner.getDeleteProjectDialogState,
    setDeleteProjectDialogState: dialogOwner.setDeleteProjectDialogState,
    getTaskFromFolderWorkflow: () => taskFromFolderWorkflow,
    closeNewProjectDialog: dialogOwner.closeNewProjectDialog,
    closeDeleteProjectDialog: dialogOwner.closeDeleteProjectDialog,
    closeAlignmentDialog: dialogOwner.closeAlignmentDialog,
    pickProjectDestination: pickProjectParentDestinationAction,
    confirmAlignment,
    confirmDeleteProject,
    setProjectsMenuOpen: (open) => {
      menuState.setProjectsMenuOpen(open);
    },
    currentProjectRoot,
    refreshProjectState,
    startProjectWatch,
    updateWindowTitleForProject,
    setActionError,
    setOperationIdle: () => {
      operationState = idleOperation;
    },
    createProjectAction,
    taskCreationWorkflow,
    taskRenameWorkflow,
    taskDeleteWorkflow,
    eventCreationWorkflow,
    eventRenameWorkflow,
    eventDeleteWorkflow,
    render,
  });

  const dispatchNavigatorIntent = async (
    intent: SidebarNavigatorIntent,
    context: SidebarNavigatorIntentContext = {},
  ) => {
    if (intent.kind === "activate-project") {
      await activateProjectFromTree(intent.projectPath);
      return;
    }
    if (intent.kind === "select-task") {
      await selectTaskFromTree(intent.projectPath, intent.taskId);
      return;
    }
    if (intent.kind === "select-event") {
      await selectEventFromTree(intent.projectPath, intent.eventId);
      return;
    }
    if (intent.kind === "toggle-project-expansion") {
      await toggleRememberedProjectExpansion(intent.projectPath);
      return;
    }
    if (intent.kind === "open-project-context-menu") {
      if (intent.source === "pointer" && context.pointerEvent) {
        await popupNativeProjectActionMenuForProject(
          projectActionMenuBindings,
          intent.projectPath,
          context.pointerEvent,
        );
        return;
      }
      openProjectActionsFallbackMenu(projectActionMenuBindings, intent.projectPath);
      return;
    }
    if (intent.kind === "open-task-context-menu") {
      openTaskContextMenu(
        intent.projectPath,
        intent.taskId,
        context.menuPosition ?? { x: 0, y: 0 },
        "sidebar",
      );
      return;
    }
    if (intent.kind === "open-event-context-menu") {
      openEventContextMenu(intent.projectPath, intent.eventId);
      return;
    }
    if (intent.kind === "run-project-command") {
      await dispatchProjectCommand(intent.projectPath, intent.command);
    }
  };

  function render() {
    const menuSnapshot = menuState.getSnapshot();
    const sessionSnapshot = projectSession.snapshot;
    visualAdapters?.renderNavigation?.({
      records: sessionSnapshot.rememberedProjects,
      projectState: sessionSnapshot.projectState,
      selectedTaskId: sessionSnapshot.selection.selectedTaskId,
      selectedEventId: sessionSnapshot.selection.selectedEventId,
      sidebarState: sessionSnapshot.sidebarState,
      projectsMenuOpen: menuSnapshot.projectsMenuOpen,
      openProjectContextMenu: menuSnapshot.openProjectContextMenu,
      openTaskCreateMenuProjectPath: menuSnapshot.openTaskCreateMenuProjectPath,
      openTaskMenu: menuSnapshot.openTaskMenu,
      openEventMenu: menuSnapshot.openEventMenu,
      onNavigatorIntent: dispatchNavigatorIntent,
      onProjectsMenuToggle: toggleProjectsMenu,
      onProjectsMenuClose: closeProjectsMenu,
      onProjectsMenuAction: async (action) => {
        if (action === "new-project") {
          openNewProjectDialog();
          return;
        }
        await openExistingProjectFromMenu();
      },
      onProjectActionMenuToggle: (projectPath) => {
        if (menuState.toggleProjectActionMenu(projectPath) === "open") {
          openProjectActionsFallbackMenu(projectActionMenuBindings, projectPath);
          return;
        }
        render();
      },
      onProjectActionMenuClose: closeProjectActionMenu,
      onTaskCreateMenuToggle: toggleTaskCreateMenu, onTaskCreateMenuClose: closeTaskCreateMenu,
      onTaskMenuClose: closeTaskContextMenu,
      onTaskAction: handleTaskContextAction,
      onEventMenuClose: closeEventContextMenu,
      onEventAction: handleEventContextAction,
    });

    visualAdapters?.renderWorkArea?.({
      workspace: buildWorkspaceSelectionModel({
        projectState: sessionSnapshot.projectState,
        selectedTaskId: sessionSnapshot.selection.selectedTaskId,
        selectedEventId: sessionSnapshot.selection.selectedEventId,
        commands: {
          ...taskInspectorWorkflow.commands(operationState),
          onEditProjectReadme: handleProjectReadmeEdit,
        },
      }),
      onSelectTask: selectTaskFromTimeline,
      onSelectEvent: selectEventFromTimeline,
      onTaskContextMenu: openTaskContextMenu,
    });

    const taskFromFolderDialogState = taskFromFolderWorkflow!.getState();
    const alignmentDialogState = projectOpenLifecycle.getAlignmentDialogState();
    const dialogSnapshot = dialogOwner.snapshot;

    visualAdapters?.renderOverlays?.({
      ...dialogSnapshot,
      taskFromFolderDialogState,
      alignmentDialogState,
      onAction: handleOverlayAction,
    });

    overlayInitialFocus.focusInitialControl({
      newProjectDialogState: dialogSnapshot.newProjectDialogState,
      taskDialogState: dialogSnapshot.taskDialogState,
      taskFromFolderDialogState,
      renameTaskDialogState: dialogSnapshot.renameTaskDialogState,
      eventDialogState: dialogSnapshot.eventDialogState,
      renameEventDialogState: dialogSnapshot.renameEventDialogState,
      deleteEventDialogState: dialogSnapshot.deleteEventDialogState,
      deleteTaskDialogState: dialogSnapshot.deleteTaskDialogState,
      deleteProjectDialogState: dialogSnapshot.deleteProjectDialogState,
    });
  }

  const openAndSelectProject = async (
    projectPath: string,
    preferredTaskId: string | null,
    preferredEventId: string | null = null,
  ) => {
    await openProjectWithAlignment(projectPath, preferredTaskId, preferredEventId);
  };

  const removeRememberedProjectReference = async (projectPath: string) => {
    await projectSession.commands.removeRememberedProject(projectPath);
    render();
  };

  const findRememberedProjectReference = async (projectPath: string) => {
    try {
      const selectedPath = await pickFolder();
      if (!selectedPath) {
        render();
        return;
      }

      const openedProject = await openSelectedProject(selectedPath);
      if (!openedProject.valid || !openedProject.projectRoot) {
        throw new Error(openedProject.issues[0] ?? `invalid project path '${selectedPath}'`);
      }

      await removeRememberedProjectReference(projectPath);
      refreshProjectState(openedProject, null, true);
      menuState.setProjectsMenuOpen(false);
      operationState = idleOperation;
      await startProjectWatch(currentProjectRoot());
      await updateWindowTitleForProject(currentProjectRoot());
    } catch (error) {
      setActionError(error);
    }
    render();
  };

  const dispatchProjectCommand = async (
    projectPath: string,
    command: ProjectActionCommand,
  ) => {
    menuState.setOpenProjectContextMenu(null);
    menuState.closeTaskCreateMenu();
    if (command.kind === "remove-remembered-project") {
      await clearActiveProjectIfMatching(projectPath);
      await removeRememberedProjectReference(projectPath);
      return;
    }
    if (command.kind === "open-delete-project-dialog") {
      const projectName = basenameFromPath(projectPath);
      dialogOwner.openDeleteProjectDialog(projectPath, projectName);
      render();
      return;
    }
    if (command.kind === "find-remembered-project") {
      await findRememberedProjectReference(projectPath);
      return;
    }
    if (command.kind === "refresh-project") {
      if (currentProjectRoot() === projectPath) {
        await refreshOpenProject();
      } else {
        await refreshRememberedProjectTasks(projectPath);
      }
      return;
    }
    if (command.kind === "reveal-project-folder") {
      try {
        await openProjectFolderAction(projectPath);
        operationState = idleOperation;
      } catch (error) {
        setActionError(error);
      }
      render();
      return;
    }
    if (command.kind === "open-create-task-dialog") {
      openCreateTaskDialog(projectPath);
      render();
      return;
    }
    if (command.kind === "open-create-task-from-folder-workflow") {
      await taskFromFolderWorkflow?.open(projectPath);
      return;
    }
    openCreateEventDialog(projectPath);
    render();
  };

  const projectActionMenuBindings = {
    root,
    getOpenProjectContextMenu: () => menuState.getOpenProjectContextMenu(),
    setOpenProjectContextMenu: (target: ProjectContextMenuTarget | null) => {
      menuState.setOpenProjectContextMenu(target);
    },
    dismissForProjectActions: () => {
      menuState.dismissForProjectActions();
      dialogOwner.closeDialogsForTaskContextMenu();
    },
    projectActionMenuEntriesForProject: (projectPath: string) =>
      projectActionEntriesForProject({
        projectPath,
        projectName: basenameFromPath(projectPath),
        missing: Boolean(projectSession.snapshot.sidebarState.scanFailures[projectPath]),
        refreshing: projectSession.snapshot.sidebarState.refreshInFlightProjectPaths.includes(
          projectPath,
        ),
      }),
    runProjectCommand: dispatchProjectCommand,
    render,
  };

  render();

  projectSession.snapshot.rememberedProjects
    .filter((record) => record.expanded)
    .forEach((record) => {
      void refreshRememberedProjectTasks(record.projectPath);
    });

  try {
    await loadHealth();
    render();
  } catch {
    root.querySelector("[data-testid='backend-status']")?.replaceChildren(
      "Backend unavailable",
    );
  }
}
