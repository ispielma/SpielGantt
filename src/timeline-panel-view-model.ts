import type { TimelinePlacedTaskView } from "./timeline-panel-types.ts";

export function shouldStaggerEventLabels(events: string[]): boolean {
  if (events.length < 4) {
    return false;
  }

  return events.some((eventId, index) => {
    const nextEventId = events[index + 1];
    if (!nextEventId) {
      return false;
    }

    return eventId.length + nextEventId.length >= 24;
  });
}

export function eventRailGridColumn(index: number, eventCount: number): string {
  if (eventCount <= 1) {
    return "1 / 2";
  }

  if (index === eventCount - 1) {
    return `${eventCount - 1} / ${eventCount}`;
  }

  return `${index + 1} / ${index + 2}`;
}

function describeBindingNodes(task: TimelinePlacedTaskView): string {
  const bindingNodeEvents = [task.startConnectorEventId, task.endConnectorEventId].filter(
    (eventId): eventId is string => Boolean(eventId),
  );

  if (bindingNodeEvents.length === 0) {
    return "";
  }

  if (bindingNodeEvents.length === 1) {
    return ` with a semantic binding node at ${bindingNodeEvents[0]}`;
  }

  return ` with semantic binding nodes at ${bindingNodeEvents.join(" and ")}`;
}

export function describeTaskAxisSpan(task: TimelinePlacedTaskView): string {
  const start = task.startEventId ?? "an unscheduled start";
  const end = task.endEventId ?? "an unscheduled end";
  const warnings = task.warningDiagnostics?.length
    ? ` Warning: ${task.warningDiagnostics.join(" ")}`
    : "";
  return `Task ${task.taskId} spans ${start} to ${end} on the event axis${describeBindingNodes(task)}${warnings}`;
}
