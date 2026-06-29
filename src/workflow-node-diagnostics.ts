import type {
  TimelineWorkflow,
  TimelineWorkflowPlacementStatus,
  TimelineWorkflowTask,
} from "./timeline.ts";

export type EventPlacementPresentationStatus = TimelineWorkflowPlacementStatus;
export type TaskDeterminationPresentationStatus = TimelineWorkflowPlacementStatus;

export function eventPlacementStatus(
  workflow: TimelineWorkflow | null,
  eventId: string,
): EventPlacementPresentationStatus {
  return workflow?.event_nodes.find((event) => event.id === eventId)?.placement_status ?? "ready";
}

export function taskDeterminationMessages(workflowTask: TimelineWorkflowTask | null): string[] {
  if (!workflowTask) {
    return ["Refresh the project to load backend workflow determination diagnostics."];
  }

  const seenMessages = new Set<string>();
  const messages: string[] = [];
  const addMessage = (message: string) => {
    if (!seenMessages.has(message)) {
      seenMessages.add(message);
      messages.push(message);
    }
  };

  for (const message of workflowTask.placement_messages) {
    addMessage(message);
  }

  if (workflowTask.placement_status === "incomplete" && messages.length === 0) {
    addMessage("Complete this task's event-axis workflow placement.");
  }

  return messages;
}

export function taskDeterminationStatus(
  workflowTask: TimelineWorkflowTask | null,
): TaskDeterminationPresentationStatus {
  if (!workflowTask) {
    return "diagnostic";
  }

  return workflowTask.placement_status;
}
