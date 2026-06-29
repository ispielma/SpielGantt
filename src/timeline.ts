import type { TaskStatus } from "./shell-types.ts";

export { buildTimelineViewProps } from "./timeline-view-model.ts";

export interface TimelineTask {
  id: string;
  dependencies: string[];
  endsAt?: string | null;
  status?: TaskStatus | null;
}

export interface TimelineWorkflow {
  schema_version: number;
  project_root: string;
  events: string[];
  event_nodes: TimelineWorkflowEvent[];
  tasks: TimelineWorkflowTask[];
  edges: Array<{
    from: TimelineWorkflowNode;
    to: TimelineWorkflowNode;
    kind: "dependency" | "ends_at";
  }>;
  validation: {
    valid: boolean;
    diagnostics: string[];
  };
}

export interface TimelineWorkflowEvent {
  id: string;
  boundary_role: "start_boundary" | "ordinary" | "finish_boundary";
  chart_order: number;
  placement_ready: boolean;
  placement_status: TimelineWorkflowPlacementStatus;
  placement_messages: string[];
}

export interface TimelineWorkflowTask {
  id: string;
  determination_status: "fully_determined" | "undetermined";
  placement_ready: boolean;
  placement_status: TimelineWorkflowPlacementStatus;
  placement_messages: string[];
  dependency_references: TimelineWorkflowDependencyReference[];
  ends_at_reference: TimelineWorkflowEndsAtReference | null;
  effective_anchors: TimelineWorkflowEffectiveAnchors;
  valid_dependency_targets: TimelineWorkflowNode[];
  valid_ends_at_targets: string[];
  unresolved_references: TimelineWorkflowReferenceIssue[];
  invalid_references: TimelineWorkflowReferenceIssue[];
  validation_diagnostics: string[];
}

export type TimelineWorkflowPlacementStatus = "ready" | "incomplete" | "diagnostic";

export interface TimelineWorkflowDependencyReference {
  id: string;
  kind: "task" | "event";
  valid: boolean;
  diagnostic?: string;
}

export interface TimelineWorkflowEndsAtReference {
  id: string;
  valid: boolean;
  diagnostic?: string;
}

export interface TimelineWorkflowEffectiveAnchors {
  upstream: string | null;
  downstream: string | null;
  diagnostics: string[];
}

export interface TimelineWorkflowNode {
  id: string;
  kind: "task" | "event";
}

export interface TimelineWorkflowReferenceIssue {
  id: string;
  kind: "dependency" | "ends_at";
}

export interface EventTimelineLayout {
  state: "unanchored" | "event-span" | "conflict";
  dependencyIds: string[];
  startEventId: string | null;
  endEventId: string | null;
  gridColumn: string | null;
  inlineStartPercent: number;
  inlineSizePercent: number;
  conflictReason: string | null;
  upstreamEventIndex: number | null;
  downstreamEventIndex: number | null;
  diagnostics: string[];
}
