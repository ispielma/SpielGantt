import type { EventInspectorPanelProps, TaskInspectorPanelProps } from "./task-inspector-panel.tsx";
import type { ProjectInspectorPanelProps } from "./project-inspector-panel.tsx";
import type {
  OpenProjectResult,
  ProjectOpenState,
  ProjectReadmeEdit,
} from "./shell-types.ts";
import type {
  TaskInspectorWorkflowCommands,
} from "./task-inspector-workflow.ts";
import { buildTimelineViewProps, type TimelineWorkflowTask } from "./timeline.ts";
import type { TimelineViewProps } from "./timeline-panel.tsx";
import { eventPlacementStatus } from "./workflow-node-diagnostics.ts";

export interface WorkspaceSelectionCommands extends TaskInspectorWorkflowCommands {
  onEditProjectReadme: (edit: ProjectReadmeEdit) => Promise<void> | void;
}

export type WorkspaceInspectorModel =
  | {
      kind: "empty";
      eyebrow: string;
      testId: string;
      title: string;
    }
  | { kind: "project"; props: ProjectInspectorPanelProps }
  | { kind: "task"; key: string; props: TaskInspectorPanelProps }
  | { kind: "event"; props: EventInspectorPanelProps };

export type WorkspaceSelectionModel =
  | {
      projectRoot: null;
      timelineProps: null;
      inspector: Extract<WorkspaceInspectorModel, { kind: "empty" }>;
    }
  | {
      projectRoot: string;
      timelineProps: TimelineViewProps;
      inspector: Exclude<WorkspaceInspectorModel, { kind: "empty" }>;
    };

export interface WorkspaceSelectionInput {
  projectState: ProjectOpenState;
  selectedTaskId: string | null;
  selectedEventId: string | null;
  commands: WorkspaceSelectionCommands;
}

function taskInspectorKey(task: OpenProjectResult["tasks"][number]) {
  return JSON.stringify({
    id: task.id,
    status: task.status,
    readmeVersion: task.readmeVersion,
    readmeContent: task.readmeContent,
    endsAt: task.endsAt ?? "",
    dependencies: task.dependencies,
    blocks: task.blocks,
    dependencyTargets: task.dependencyTargets,
  });
}

function workflowTaskForTask(
  project: OpenProjectResult,
  taskId: string,
): TimelineWorkflowTask | null {
  return (
    project.workflow?.tasks.find((workflowTask) => workflowTask.id === taskId)
      ?? null
  );
}

function emptyWorkspaceModel(
  reason: "no-project" | "project-unavailable",
): WorkspaceSelectionModel {
  return {
    projectRoot: null,
    timelineProps: null,
    inspector: reason === "no-project"
      ? {
          kind: "empty",
          eyebrow: "No project open",
          testId: "project-empty-state",
          title: "Choose a SpielGantt project folder to begin.",
        }
      : {
          kind: "empty",
          eyebrow: "Project unavailable",
          testId: "project-empty-state",
          title: "No SpielGantt project metadata was found.",
        },
  };
}

function taskInspectorProps(
  project: OpenProjectResult,
  task: OpenProjectResult["tasks"][number],
  commands: WorkspaceSelectionCommands,
): Extract<WorkspaceInspectorModel, { kind: "task" }> {
  const workflowTask = workflowTaskForTask(project, task.id);
  return {
    kind: "task",
    key: taskInspectorKey(task),
    props: {
      task,
      workflowTask,
      validEndEventTargets: workflowTask?.valid_ends_at_targets ?? [],
      operationState: commands.operationState,
      taskEndsAtControlState: commands.taskEndsAtControlState,
      taskDependencyControlState: commands.taskDependencyControlState,
      onEditTask: commands.onEditTask,
      onSetTaskEndsAt: commands.onSetTaskEndsAt,
      onSelectTaskEndsAtEvent: commands.onSelectTaskEndsAtEvent,
      onCreateTaskEndEvent: commands.onCreateTaskEndEvent,
      onSelectTaskDependencyBlocker: commands.onSelectTaskDependencyBlocker,
      onAddDependency: commands.onAddDependency,
      onRemoveDependency: commands.onRemoveDependency,
    },
  };
}

function eventInspectorProps(
  project: OpenProjectResult,
  eventId: string,
): Extract<WorkspaceInspectorModel, { kind: "event" }> {
  const eventReference = project.eventReferences.find((event) => event.id === eventId);
  return {
    kind: "event",
    props: {
      eventId,
      blockerTaskIds: eventReference?.blockerTaskIds ?? [],
      blockedTaskIds: eventReference?.blockedTaskIds ?? [],
      placementStatus: eventPlacementStatus(project.workflow, eventId),
    },
  };
}

export function buildWorkspaceSelectionModel(
  input: WorkspaceSelectionInput,
): WorkspaceSelectionModel {
  const { projectState, selectedTaskId, selectedEventId, commands } = input;
  if (projectState.status === "none") {
    return emptyWorkspaceModel("no-project");
  }

  const { project } = projectState;
  if (!project.projectRoot) {
    return emptyWorkspaceModel("project-unavailable");
  }

  const selectedEventIdForProject =
    selectedEventId && project.events.includes(selectedEventId)
      ? selectedEventId
      : null;
  const selectedTask =
    !selectedEventIdForProject && selectedTaskId
      ? project.tasks.find((task) => task.id === selectedTaskId) ?? null
      : null;

  const timelineProps = {
    ...buildTimelineViewProps(project.tasks, project.workflow),
    selectedTaskId: selectedTask?.id ?? null,
    selectedEventId: selectedEventIdForProject,
  };

  if (selectedEventIdForProject) {
    return {
      projectRoot: project.projectRoot,
      timelineProps,
      inspector: eventInspectorProps(project, selectedEventIdForProject),
    };
  }

  if (selectedTask) {
    return {
      projectRoot: project.projectRoot,
      timelineProps,
      inspector: taskInspectorProps(project, selectedTask, commands),
    };
  }

  return {
    projectRoot: project.projectRoot,
    timelineProps,
    inspector: {
      kind: "project",
      props: {
        project,
        onEditProjectReadme: commands.onEditProjectReadme,
      },
    },
  };
}
