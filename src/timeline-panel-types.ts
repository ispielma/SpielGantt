import type { TaskStatus } from "./shell-types.ts";

export interface TimelinePlacedTaskView {
  taskId: string;
  layoutState: string;
  status?: TaskStatus | null;
  warningDiagnostics?: string[];
  dependencyIds: string[];
  startEventId: string | null;
  endEventId: string | null;
  gridColumn: string;
  inlineStartPercent: number;
  inlineSizePercent: number;
  startConnectorEventId: string | null;
  endConnectorEventId: string | null;
}

export interface TimelineStatusTaskView {
  taskId: string;
  message: string;
  conflictReason?: string | null;
}

export interface TimelineViewProps {
  hasEvents: boolean;
  events: string[];
  eventRailColumns: string;
  placedTasks: TimelinePlacedTaskView[];
  unanchoredTasks: TimelineStatusTaskView[];
  layoutConflicts: TimelineStatusTaskView[];
  selectedTaskId?: string | null;
  selectedEventId?: string | null;
}
