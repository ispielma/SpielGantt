import { basenameFromPath } from "./project-tree-dom.ts";
import type { RememberedProjectRecord } from "./remembered-projects.ts";
import {
  emptyTaskFolderAlignmentDialogState,
  type TaskFolderAlignmentDialogState,
} from "./shell-dialogs.tsx";
import { resolveRememberedTaskId } from "./shell-project-session.ts";
import type {
  OpenProjectResult,
  ProjectChangeSubscriber,
  ProjectOpener,
  ProjectWatchSubscription,
  TaskFolderAlignmentApplier,
  TaskFolderAlignmentPreviewer,
  WindowTitleSetter,
} from "./shell-types.ts";

interface OpenProjectRequest {
  projectPath: string;
  preferredTaskId?: string | null;
  preferredEventId?: string | null;
  prepareProject?: ProjectOpener | null;
}

interface ProjectOpenLifecycleSession {
  currentProjectRoot: () => string | null;
  rememberedProjects: () => RememberedProjectRecord[];
  activateProject: (
    project: OpenProjectResult,
    preferredTaskId?: string | null,
    expandRememberedRecord?: boolean | null,
    preferredEventId?: string | null,
  ) => void;
  selectEventIfPresent: (project: OpenProjectResult, eventId: string | null) => void;
  clearActiveProject: () => void;
  setProjectsMenuOpen: (open: boolean) => void;
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

interface ProjectOpenLifecycleDeps {
  openSelectedProject: ProjectOpener;
  previewAlignmentAction: TaskFolderAlignmentPreviewer;
  applyAlignmentAction: TaskFolderAlignmentApplier;
  subscribeProjectSessionChanges: ProjectChangeSubscriber;
  setWindowTitle: WindowTitleSetter;
}

export class ProjectOpenLifecycle {
  private alignmentDialogState: TaskFolderAlignmentDialogState = {
    ...emptyTaskFolderAlignmentDialogState,
  };
  private pendingAlignedProjectTaskSelection: string | null = null;
  private pendingAlignedProjectEventSelection: string | null = null;
  private projectSessionWatchSubscription: ProjectWatchSubscription | null = null;

  constructor(
    private readonly deps: ProjectOpenLifecycleDeps,
    private readonly session: ProjectOpenLifecycleSession,
  ) {}

  getAlignmentDialogState(): TaskFolderAlignmentDialogState {
    return this.alignmentDialogState;
  }

  closeAlignmentDialog(): void {
    this.alignmentDialogState = { ...emptyTaskFolderAlignmentDialogState };
    this.pendingAlignedProjectTaskSelection = null;
    this.pendingAlignedProjectEventSelection = null;
  }

  async stopProjectWatch(): Promise<void> {
    const subscription = this.projectSessionWatchSubscription;
    this.projectSessionWatchSubscription = null;
    await subscription?.unsubscribe();
  }

  async refreshOpenProject(): Promise<void> {
    const projectRoot = this.session.currentProjectRoot();
    if (!projectRoot) {
      return;
    }

    try {
      const refreshedProject = await this.deps.openSelectedProject(projectRoot);
      this.session.activateProject(refreshedProject);
      this.session.setOperationIdle();
    } catch (error) {
      this.session.setActionError(error);
    }
    this.session.render();
  }

  async startProjectWatch(projectRoot: string | null): Promise<void> {
    await this.stopProjectWatch();
    if (!projectRoot) {
      return;
    }

    try {
      this.projectSessionWatchSubscription =
        await this.deps.subscribeProjectSessionChanges(
        projectRoot,
        () => this.refreshOpenProject(),
      );
    } catch (error) {
      void error;
    }
  }

  async updateWindowTitleForProject(projectRoot: string | null): Promise<void> {
    await this.deps.setWindowTitle(
      projectRoot ? `SpielGantt - ${basenameFromPath(projectRoot)}` : "SpielGantt",
    );
  }

  async clearActiveProjectIfMatching(projectPath: string): Promise<void> {
    if (this.session.currentProjectRoot() !== projectPath) {
      return;
    }

    await this.stopProjectWatch();
    this.session.clearActiveProject();
    await this.updateWindowTitleForProject(null);
  }

  async openProjectWithAlignment({
    projectPath,
    preferredTaskId = null,
    preferredEventId = null,
    prepareProject = null,
  }: OpenProjectRequest): Promise<void> {
    try {
      const preparedProject = prepareProject ? await prepareProject(projectPath) : null;
      const alignedProjectPath = preparedProject?.projectRoot ?? projectPath;
      const projectName = basenameFromPath(alignedProjectPath);
      const plan = await this.deps.previewAlignmentAction(alignedProjectPath);
      if (plan.operations.length === 0 && plan.preflightIssues.length === 0) {
        const openedProject =
          preparedProject ?? await this.deps.openSelectedProject(alignedProjectPath);
        const initialTaskSelection =
          preferredEventId
            ? null
            : preferredTaskId ??
              resolveRememberedTaskId(openedProject, this.session.rememberedProjects());
        this.session.activateProject(openedProject, initialTaskSelection, true, preferredEventId);
        this.session.setProjectsMenuOpen(false);
        this.session.setOperationIdle();
        await this.startProjectWatch(this.session.currentProjectRoot());
        await this.updateWindowTitleForProject(this.session.currentProjectRoot());
        this.session.render();
        return;
      }

      this.session.setProjectsMenuOpen(false);
      this.pendingAlignedProjectTaskSelection = preferredTaskId;
      this.pendingAlignedProjectEventSelection = preferredEventId;
      this.alignmentDialogState = {
        open: true,
        projectPath: alignedProjectPath,
        projectName,
        plan,
        errorMessage: null,
        submitting: false,
      };
      this.session.render();
    } catch (error) {
      this.session.setActionError(error);
      this.session.render();
    }
  }

  async confirmAlignment(): Promise<void> {
    const plan = this.alignmentDialogState.plan;
    const projectPath = this.alignmentDialogState.projectPath;
    if (!this.alignmentDialogState.open || !plan || plan.preflightIssues.length > 0) {
      return;
    }

    this.alignmentDialogState = {
      ...this.alignmentDialogState,
      submitting: true,
      errorMessage: null,
    };
    this.session.render();

    try {
      const result = await this.deps.applyAlignmentAction(projectPath, plan);
      const alignedProject = result.project;
      this.session.activateProject(
        alignedProject,
        this.pendingAlignedProjectTaskSelection,
        true,
      );
      this.session.selectEventIfPresent(
        alignedProject,
        this.pendingAlignedProjectEventSelection,
      );
      this.session.setOperationIdle();
      this.closeAlignmentDialog();
      await this.startProjectWatch(this.session.currentProjectRoot());
      await this.updateWindowTitleForProject(this.session.currentProjectRoot());
    } catch (error) {
      this.alignmentDialogState = {
        ...this.alignmentDialogState,
        submitting: false,
        errorMessage: error instanceof Error ? error.message : String(error),
      };
      this.session.setActionError(error);
    }
    this.session.render();
  }
}
