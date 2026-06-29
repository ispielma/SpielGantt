import type { MouseEvent } from "react";

import { EmptyWorkspacePanel } from "./inspector-components.tsx";
import { TimelineView } from "./timeline-panel.tsx";
import {
  EventInspectorPanel,
  TaskInspectorPanel,
} from "./task-inspector-panel.tsx";
import { ProjectInspectorPanel } from "./project-inspector-panel.tsx";
import type { WorkspaceSelectionModel } from "./workspace-selection-model.ts";

export {
  emptyTaskDependencyControlState,
  emptyTaskEndsAtControlState,
  type TaskDependencyControlState,
  type TaskEndsAtControlState,
} from "./task-inspector-workflow.ts";

interface ProjectWorkspaceProps {
  workspace: WorkspaceSelectionModel;
  onSelectTask: (taskId: string) => void;
  onSelectEvent: (eventId: string) => void;
  onTaskContextMenu: (
    projectPath: string,
    taskId: string,
    position: { x: number; y: number },
    source?: "sidebar" | "timeline",
  ) => void;
}

export function ProjectWorkspace(props: ProjectWorkspaceProps) {
  const {
    workspace,
    onSelectTask,
    onSelectEvent,
    onTaskContextMenu,
  } = props;

  if (workspace.timelineProps === null) {
    return (
      <EmptyWorkspacePanel
        eyebrow={workspace.inspector.eyebrow}
        testId={workspace.inspector.testId}
        title={workspace.inspector.title}
      />
    );
  }

  const handleTimelineClick = (event: MouseEvent<HTMLElement>) => {
    const eventTarget = (event.target as HTMLElement | null)?.closest<HTMLElement>(
      "[data-timeline-event-select='true']",
    );
    const eventId = eventTarget?.dataset.eventId ?? "";
    if (eventId) {
      onSelectEvent(eventId);
      return;
    }

    const taskTarget = (event.target as HTMLElement | null)?.closest<HTMLElement>(
      "[data-timeline-task-select='true']",
    );
    const taskId = taskTarget?.dataset.taskId ?? "";
    if (taskId) {
      onSelectTask(taskId);
    }
  };

  const handleTimelineContextMenu = (event: MouseEvent<HTMLElement>) => {
    const target = (event.target as HTMLElement | null)?.closest<HTMLElement>(
      "[data-timeline-task-select='true']",
    );
    const taskId = target?.dataset.taskId ?? "";
    if (taskId) {
      event.preventDefault();
      onTaskContextMenu(
        workspace.projectRoot,
        taskId,
        {
          x: event.clientX,
          y: event.clientY,
        },
        "timeline",
      );
    }
  };

  return (
    <section className="project-workspace" aria-label="Project workspace">
      <div onClick={handleTimelineClick} onContextMenu={handleTimelineContextMenu}>
        <TimelineView {...workspace.timelineProps} />
      </div>
      {workspace.inspector.kind === "event" ? (
        <EventInspectorPanel {...workspace.inspector.props} />
      ) : workspace.inspector.kind === "task" ? (
        <TaskInspectorPanel
          key={workspace.inspector.key}
          {...workspace.inspector.props}
        />
      ) : (
        <ProjectInspectorPanel {...workspace.inspector.props} />
      )}
    </section>
  );
}
