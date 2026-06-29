import {
  type TimelineViewProps,
  type TimelineStatusTaskView,
  type TimelinePlacedTaskView,
} from "./timeline-panel-types.ts";
import {
  buildVisualTimelineRows,
  type VisualTimelineRow,
} from "./timeline-placement.ts";
import { orderedTimelineEvents } from "./timeline-event-order.ts";
import type { EventTimelineLayout, TimelineTask, TimelineWorkflow } from "./timeline.ts";

function dependsOnTask(
  rowsByTaskId: Map<string, VisualTimelineRow>,
  taskId: string,
  possibleDependencyId: string,
  seenTaskIds = new Set<string>(),
): boolean {
  if (seenTaskIds.has(taskId)) return false;

  seenTaskIds.add(taskId);
  const row = rowsByTaskId.get(taskId);
  if (!row) return false;

  return row.taskDependencyIds.some(
    (dependencyId) =>
      dependencyId === possibleDependencyId ||
      dependsOnTask(rowsByTaskId, dependencyId, possibleDependencyId, seenTaskIds),
  );
}

function comparePlacedRows(
  rowsByTaskId: Map<string, VisualTimelineRow>,
  left: { task: TimelineTask; layout: EventTimelineLayout },
  right: { task: TimelineTask; layout: EventTimelineLayout },
): number {
  // Keep earlier/blocking work higher so event-axis bars cascade down like a Gantt chart.
  if (dependsOnTask(rowsByTaskId, left.task.id, right.task.id)) return 1;
  if (dependsOnTask(rowsByTaskId, right.task.id, left.task.id)) return -1;

  return (
    (left.layout.upstreamEventIndex ?? Number.MAX_SAFE_INTEGER) -
      (right.layout.upstreamEventIndex ?? Number.MAX_SAFE_INTEGER) ||
    (left.layout.downstreamEventIndex ?? Number.MAX_SAFE_INTEGER) -
      (right.layout.downstreamEventIndex ?? Number.MAX_SAFE_INTEGER) ||
    left.task.id.localeCompare(right.task.id)
  );
}

function describeEventTimelineConflict(
  task: TimelineTask,
  layout: EventTimelineLayout,
): string {
  if (layout.diagnostics.length) {
    return layout.diagnostics.join(" ");
  }

  if (layout.conflictReason === "reversed-event-span") {
    return `Task ${task.id} has conflicting event context: ${layout.startEventId ?? "unknown"} comes after ${layout.endEventId ?? "unknown"}.`;
  }

  return `Task ${task.id} has a layout conflict on the event axis.`;
}

function describeIncompleteTimelinePlacement(layout: EventTimelineLayout): string {
  const messages = new Set<string>();

  for (const diagnostic of layout.diagnostics) {
    messages.add(diagnostic);
  }

  if (layout.startEventId === null) {
    messages.add("Add a dependency target or event anchor for the task start.");
  }

  if (layout.endEventId === null) {
    messages.add("Choose an end event for this task.");
  }

  return Array.from(messages).join(" ") || "Not placed on event axis";
}

const MIN_EVENT_INTERVAL_WIDTH_PX = 120;
const TASK_LABEL_CHARACTER_WIDTH_PX = 8;
const TASK_LABEL_CHROME_WIDTH_PX = 44;

function taskLabelWidthPx(taskId: string): number {
  return Math.ceil(taskId.length * TASK_LABEL_CHARACTER_WIDTH_PX + TASK_LABEL_CHROME_WIDTH_PX);
}

function buildEventRailColumns(
  events: string[],
  tasks: Array<{ task: TimelineTask; layout: EventTimelineLayout }>,
): string {
  const intervalCount = Math.max(events.length - 1, 1);
  const intervalWidths = Array.from({ length: intervalCount }, () => MIN_EVENT_INTERVAL_WIDTH_PX);

  for (const { task, layout } of tasks) {
    const startIndex = layout.upstreamEventIndex;
    const endIndex = layout.downstreamEventIndex;
    if (startIndex === null || endIndex === null || endIndex <= startIndex) {
      continue;
    }

    const span = endIndex - startIndex;
    const inlineFraction = Math.max(layout.inlineSizePercent / 100, 0.1);
    const requiredTaskWidth = Math.ceil(taskLabelWidthPx(task.id) / inlineFraction);
    const requiredIntervalWidth = Math.ceil(requiredTaskWidth / span);

    for (let intervalIndex = startIndex; intervalIndex < endIndex; intervalIndex += 1) {
      intervalWidths[intervalIndex] = Math.max(
        intervalWidths[intervalIndex],
        requiredIntervalWidth,
      );
    }
  }

  return intervalWidths.map((width) => `${width}px`).join(" ");
}

export function buildTimelineViewProps(
  tasks: TimelineTask[],
  workflow: TimelineWorkflow | null = null,
): TimelineViewProps {
  const events = orderedTimelineEvents(workflow);
  if (events.length === 0) {
    return {
      hasEvents: false,
      events,
      eventRailColumns: "",
      placedTasks: [],
      unanchoredTasks: tasks.map(
        (task): TimelineStatusTaskView => ({
          taskId: task.id,
          message: "Add project events before placing this task on the timeline.",
        }),
      ),
      layoutConflicts: [],
    };
  }

  const visualRows = buildVisualTimelineRows(workflow);
  const visualRowsByTaskId = new Map(visualRows.map((row) => [row.taskId, row]));
  const tasksById = new Map(tasks.map((task) => [task.id, task]));
  const providedEventTimelineTasks: Array<{ task: TimelineTask; layout: EventTimelineLayout }> =
    visualRows.flatMap((layout) => {
      const task = tasksById.get(layout.taskId);
      if (!task) {
        return [];
      }
      return [{ task, layout }];
    });
  const eventTimelineTasks = providedEventTimelineTasks;
  const placedTasks = eventTimelineTasks
    .filter(({ layout }) => layout.state === "event-span")
    .sort((left, right) => comparePlacedRows(visualRowsByTaskId, left, right));
  const unanchoredTasks = eventTimelineTasks.filter(({ layout }) => layout.state === "unanchored");
  const layoutConflicts = eventTimelineTasks.filter(({ layout }) => layout.state === "conflict");
  const eventRailColumns = buildEventRailColumns(events, placedTasks);
  return {
    hasEvents: true,
    events,
    eventRailColumns,
    placedTasks: placedTasks.map(
      ({ task, layout }): TimelinePlacedTaskView => ({
        taskId: task.id,
        layoutState: layout.state,
        status: task.status ?? "unblocked",
        warningDiagnostics: layout.diagnostics,
        dependencyIds: layout.dependencyIds,
        startEventId: layout.startEventId,
        endEventId: layout.endEventId,
        gridColumn: layout.gridColumn ?? "auto",
        startConnectorEventId: layout.dependencyIds.length > 0 ? layout.startEventId : null,
        endConnectorEventId: layout.endEventId,
        inlineStartPercent: layout.inlineStartPercent,
        inlineSizePercent: layout.inlineSizePercent,
      }),
    ),
    unanchoredTasks: unanchoredTasks.map(
      ({ task, layout }): TimelineStatusTaskView => ({
        taskId: task.id,
        message: describeIncompleteTimelinePlacement(layout),
      }),
    ),
    layoutConflicts: layoutConflicts.map(
      ({ task, layout }): TimelineStatusTaskView => ({
        taskId: task.id,
        message: describeEventTimelineConflict(task, layout),
        conflictReason: layout.conflictReason ?? "event span",
      }),
    ),
  };
}
