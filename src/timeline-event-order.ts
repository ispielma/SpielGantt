import type { TimelineWorkflow, TimelineWorkflowEvent } from "./timeline.ts";

const BOUNDARY_ROLE_ORDER: Record<TimelineWorkflowEvent["boundary_role"], number> = {
  start_boundary: 0,
  ordinary: 1,
  finish_boundary: 2,
};

interface OrderableTimelineEvent extends TimelineWorkflowEvent {
  sourceIndex: number;
}

function eventNodesFromContract(workflow: TimelineWorkflow): OrderableTimelineEvent[] {
  return workflow.event_nodes.map((event, sourceIndex) => ({
    ...event,
    sourceIndex,
  }));
}

export function orderedTimelineEvents(workflow: TimelineWorkflow | null): string[] {
  if (!workflow) return [];

  const events = eventNodesFromContract(workflow);

  return [...events]
    .sort(
      (left, right) =>
        BOUNDARY_ROLE_ORDER[left.boundary_role] - BOUNDARY_ROLE_ORDER[right.boundary_role] ||
        left.chart_order - right.chart_order ||
        left.sourceIndex - right.sourceIndex,
    )
    .map((event) => event.id);
}
