import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { SimpleGrid, Text } from "@mantine/core";

import {
  InspectorSurface,
  InspectorTokenList,
} from "./inspector-components.tsx";
import type { OperationState, ProjectTask, TaskEdit } from "./shell-types.ts";
import {
  taskEditDraftFromTask,
  taskEditFromDraft,
  TaskDependenciesSection,
  TaskMetadataSection,
  TaskOperationStatus,
  TaskReadmeSection,
} from "./task-inspector-sections.tsx";
import {
  measureTaskInspectorLayout,
  type TaskInspectorLayout,
} from "./task-inspector-layout.ts";
import type { TimelineWorkflowTask } from "./timeline.ts";
import {
  taskDeterminationMessages,
  taskDeterminationStatus,
  type EventPlacementPresentationStatus,
} from "./workflow-node-diagnostics.ts";
import type {
  TaskDependencyControlState,
  TaskEndsAtControlState,
} from "./task-inspector-workflow.ts";

export interface TaskInspectorPanelProps {
  task: ProjectTask;
  workflowTask: TimelineWorkflowTask | null;
  validEndEventTargets: string[];
  operationState: OperationState;
  taskEndsAtControlState: TaskEndsAtControlState;
  taskDependencyControlState: TaskDependencyControlState;
  onEditTask: (taskId: string, edit: TaskEdit) => Promise<void> | void;
  onSetTaskEndsAt: (taskId: string, eventId: string | null, clear: boolean) => Promise<void> | void;
  onSelectTaskEndsAtEvent: (eventId: string) => void;
  onCreateTaskEndEvent: (taskId: string) => void;
  onSelectTaskDependencyBlocker: (blockerId: string) => void;
  onAddDependency: (taskId: string, blockerId: string) => Promise<void> | void;
  onRemoveDependency: (taskId: string, blockerId: string) => Promise<void> | void;
}

export interface EventInspectorPanelProps {
  eventId: string;
  blockerTaskIds: string[];
  blockedTaskIds: string[];
  placementStatus?: EventPlacementPresentationStatus;
}

export function TaskInspectorPanel(props: TaskInspectorPanelProps) {
  const {
    task,
    workflowTask,
    validEndEventTargets,
    operationState,
    taskEndsAtControlState,
    taskDependencyControlState,
    onEditTask,
    onSetTaskEndsAt,
    onSelectTaskEndsAtEvent,
    onCreateTaskEndEvent,
    onSelectTaskDependencyBlocker,
    onAddDependency,
    onRemoveDependency,
  } = props;
  const [draft, setDraft] = useState(() => taskEditDraftFromTask(task));
  const acceptedDraftRef = useRef(draft);
  const inspectorRef = useRef<HTMLElement | null>(null);
  const [layout, setLayout] = useState<TaskInspectorLayout>("stacked");

  useEffect(() => {
    const acceptedDraft = taskEditDraftFromTask(task);
    acceptedDraftRef.current = acceptedDraft;
    setDraft(acceptedDraft);
  }, [task.id, task.status, task.readmeContent, task.readmeVersion]);

  const persistDraft = async (nextDraft: ReturnType<typeof taskEditDraftFromTask>) => {
    const previousAcceptedDraft = acceptedDraftRef.current;
    setDraft(nextDraft);
    try {
      await onEditTask(task.id, taskEditFromDraft(task, nextDraft));
      acceptedDraftRef.current = nextDraft;
    } catch {
      setDraft(previousAcceptedDraft);
    }
  };

  useLayoutEffect(() => {
    const surface = inspectorRef.current;
    const ownerWindow = surface?.ownerDocument.defaultView;
    if (!surface || !ownerWindow) {
      return;
    }

    let cancelled = false;
    const syncLayout = () => {
      if (!cancelled) {
        setLayout(measureTaskInspectorLayout(surface));
      }
    };

    syncLayout();
    const ResizeObserverCtor = ownerWindow.ResizeObserver;
    const observer = ResizeObserverCtor ? new ResizeObserverCtor(syncLayout) : null;
    observer?.observe(surface);
    ownerWindow.addEventListener("resize", syncLayout);
    void surface.ownerDocument.fonts?.ready.then(syncLayout);

    return () => {
      cancelled = true;
      observer?.disconnect();
      ownerWindow.removeEventListener("resize", syncLayout);
    };
  }, [task.id, task.dependencies.length, validEndEventTargets.length]);
  const determinationStatus = taskDeterminationStatus(workflowTask);
  const determinationMessages = taskDeterminationMessages(workflowTask);
  const showDeterminationMarker =
    determinationStatus !== "ready"
    || determinationMessages.length > 0;
  const titleDeterminationStatus =
    determinationStatus === "ready" && determinationMessages.length > 0
      ? "diagnostic"
      : determinationStatus;

  return (
    <InspectorSurface
      aria-label="Task inspector"
      className="task-inspector"
      contentClassName="task-inspector-layout"
      contentLayout={layout}
      contentTestId="task-inspector-layout"
      surfaceRef={inspectorRef}
      testId="task-inspector"
    >
      <Text
        className="inspector-title"
        component="h2"
        data-placement-status={showDeterminationMarker ? titleDeterminationStatus : undefined}
        data-testid="task-inspector-title"
        fw={700}
        size="xl"
      >
        {showDeterminationMarker ? (
          <span aria-hidden="true" className="inspector-title-diagnostic-marker">
            !
          </span>
        ) : null}
        {task.id}
      </Text>
      <TaskMetadataSection
        draft={draft}
        onPersistDraft={persistDraft}
      />
      <TaskReadmeSection
        draft={draft}
        onChangeDraft={setDraft}
        onPersistDraft={persistDraft}
      />
      <TaskDependenciesSection
        task={task}
        workflowTask={workflowTask}
        validEndEventTargets={validEndEventTargets}
        taskEndsAtControlState={taskEndsAtControlState}
        taskDependencyControlState={taskDependencyControlState}
        onSetTaskEndsAt={onSetTaskEndsAt}
        onSelectTaskEndsAtEvent={onSelectTaskEndsAtEvent}
        onCreateEndEvent={onCreateTaskEndEvent}
        onSelectTaskDependencyBlocker={onSelectTaskDependencyBlocker}
        onAddDependency={onAddDependency}
        onRemoveDependency={onRemoveDependency}
      />
      <TaskOperationStatus operationState={operationState} />
    </InspectorSurface>
  );
}

function EventRelationshipGroup(props: {
  label: string;
  items: string[];
  emptyMessage: string;
}) {
  return (
    <section aria-label={props.label} className="event-relationship-group" role="group">
      <Text className="event-relationship-title" component="h3" fw={500}>
        {props.label}
      </Text>
      <InspectorTokenList emptyMessage={props.emptyMessage} items={props.items} tone="accent" />
    </section>
  );
}

export function EventInspectorPanel(props: EventInspectorPanelProps) {
  const { eventId, blockerTaskIds, blockedTaskIds, placementStatus = "ready" } = props;
  return (
    <InspectorSurface
      aria-label="Event inspector"
      className="event-inspector"
      testId="event-inspector"
    >
      <Text
        className="inspector-title"
        component="h2"
        data-placement-status={placementStatus === "ready" ? undefined : placementStatus}
        fw={700}
        size="xl"
      >
        {placementStatus === "ready" ? null : (
          <span aria-hidden="true" className="inspector-title-diagnostic-marker">
            !
          </span>
        )}
        {eventId}
      </Text>
      <SimpleGrid className="event-inspector-relationships" cols={{ base: 1, md: 2 }} spacing="md">
        <EventRelationshipGroup
          label="Before this event"
          items={blockerTaskIds}
          emptyMessage="No tasks arrive at this event yet."
        />
        <EventRelationshipGroup
          label="After this event"
          items={blockedTaskIds}
          emptyMessage="No tasks continue after this event yet."
        />
      </SimpleGrid>
    </InspectorSurface>
  );
}
