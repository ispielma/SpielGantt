import type {
  EventTimelineLayout,
  TimelineWorkflow,
  TimelineWorkflowDependencyReference,
  TimelineWorkflowTask,
} from "./timeline.ts";
import { orderedTimelineEvents } from "./timeline-event-order.ts";

export interface VisualTimelineRow extends EventTimelineLayout {
  taskId: string;
  taskDependencyIds: string[];
}

function validEventDependencies(
  task: TimelineWorkflowTask,
  eventIndexById: Map<string, number>,
): TimelineWorkflowDependencyReference[] {
  return task.dependency_references
    .filter(
      (dependency) =>
        dependency.kind === "event" && dependency.valid && eventIndexById.has(dependency.id),
    )
    .sort((left, right) => (eventIndexById.get(left.id) ?? 0) - (eventIndexById.get(right.id) ?? 0));
}

function validTaskDependencyIds(task: TimelineWorkflowTask): string[] {
  return task.dependency_references
    .filter((dependency) => dependency.kind === "task" && dependency.valid)
    .map((dependency) => dependency.id);
}

function timelineDiagnostics(task: TimelineWorkflowTask): string[] {
  return Array.from(new Set(task.placement_messages));
}

function applyTaskChainFractions(rows: VisualTimelineRow[]): VisualTimelineRow[] {
  const rowsByTaskId = new Map(rows.map((row) => [row.taskId, row]));
  const rowsByEventInterval = new Map<string, VisualTimelineRow[]>();

  for (const row of rows) {
    if (
      row.state !== "event-span" ||
      row.upstreamEventIndex === null ||
      row.downstreamEventIndex === null ||
      row.downstreamEventIndex - row.upstreamEventIndex !== 1
    ) {
      continue;
    }

    const intervalKey = `${row.upstreamEventIndex}:${row.downstreamEventIndex}`;
    rowsByEventInterval.set(intervalKey, [...(rowsByEventInterval.get(intervalKey) ?? []), row]);
  }

  const fractionsByTaskId = new Map<string, { start: number; size: number }>();

  for (const intervalRows of rowsByEventInterval.values()) {
    const intervalTaskIds = new Set(intervalRows.map((row) => row.taskId));
    const sameIntervalDependencyIds = (row: VisualTimelineRow) =>
      row.taskDependencyIds.filter((dependencyId) => intervalTaskIds.has(dependencyId));
    const hasSameIntervalDependent = new Set<string>();

    for (const row of intervalRows) {
      for (const dependencyId of sameIntervalDependencyIds(row)) {
        hasSameIntervalDependent.add(dependencyId);
      }
    }

    const positionByTaskId = new Map<string, number>();
    const positionFor = (row: VisualTimelineRow, seenTaskIds = new Set<string>()): number => {
      const cached = positionByTaskId.get(row.taskId);
      if (cached !== undefined) return cached;
      if (seenTaskIds.has(row.taskId)) return 0;

      seenTaskIds.add(row.taskId);
      const position =
        Math.max(
          -1,
          ...sameIntervalDependencyIds(row).map((dependencyId) => {
            const dependencyRow = rowsByTaskId.get(dependencyId);
            return dependencyRow ? positionFor(dependencyRow, seenTaskIds) : -1;
          }),
        ) + 1;
      positionByTaskId.set(row.taskId, position);
      return position;
    };

    const chainRows = intervalRows.filter(
      (row) =>
        sameIntervalDependencyIds(row).length > 0 || hasSameIntervalDependent.has(row.taskId),
    );
    if (chainRows.length === 0) {
      continue;
    }

    const chainLength = Math.max(...chainRows.map((row) => positionFor(row))) + 1;
    if (chainLength <= 1) {
      continue;
    }

    for (const row of chainRows) {
      const segmentSize = 100 / chainLength;
      fractionsByTaskId.set(row.taskId, {
        start: segmentSize * positionFor(row),
        size: segmentSize,
      });
    }
  }

  return rows.map((row) => {
    const fraction = fractionsByTaskId.get(row.taskId);
    return fraction
      ? { ...row, inlineStartPercent: fraction.start, inlineSizePercent: fraction.size }
      : row;
  });
}

export function buildVisualTimelineRows(workflow: TimelineWorkflow | null): VisualTimelineRow[] {
  if (!workflow) return [];

  const events = orderedTimelineEvents(workflow);
  const eventIndexById = new Map(events.map((eventId, index) => [eventId, index]));

  const rows = workflow.tasks.map((task): VisualTimelineRow => {
    const eventDependencies = validEventDependencies(task, eventIndexById);
    const taskDependencyIds = validTaskDependencyIds(task);
    const startEventId = task.effective_anchors.upstream;
    const endEventId = task.effective_anchors.downstream;
    const startEventIndex = startEventId ? eventIndexById.get(startEventId) ?? null : null;
    const endEventIndex = endEventId ? eventIndexById.get(endEventId) ?? null : null;
    const canDrawEventSpan =
      startEventIndex !== null && endEventIndex !== null && startEventIndex < endEventIndex;
    const hasReversedEventSpan =
      startEventIndex !== null && endEventIndex !== null && startEventIndex > endEventIndex;
    const hasSameEventAnchors =
      startEventIndex !== null && endEventIndex !== null && startEventIndex === endEventIndex;
    const hasWorkflowDiagnostic = task.placement_status === "diagnostic";

    return {
      taskId: task.id,
      taskDependencyIds,
      state: canDrawEventSpan
        ? "event-span"
        : hasReversedEventSpan || hasSameEventAnchors || hasWorkflowDiagnostic
          ? "conflict"
          : "unanchored",
      dependencyIds: eventDependencies.map((dependency) => dependency.id),
      startEventId,
      endEventId,
      gridColumn: canDrawEventSpan ? `${startEventIndex + 1} / ${endEventIndex + 1}` : null,
      inlineStartPercent: 0,
      inlineSizePercent: 100,
      conflictReason:
        hasReversedEventSpan
          ? "reversed-event-span"
          : hasSameEventAnchors
            ? "same-event-anchors"
            : hasWorkflowDiagnostic
              ? "workflow-diagnostic"
              : null,
      upstreamEventIndex: startEventIndex,
      downstreamEventIndex: endEventIndex,
      diagnostics: timelineDiagnostics(task),
    };
  });

  return applyTaskChainFractions(rows);
}
