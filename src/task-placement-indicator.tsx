import { List, Stack, Text } from "@mantine/core";

import type { TimelineWorkflowTask } from "./timeline.ts";
import {
  taskDeterminationMessages,
  taskDeterminationStatus,
  type TaskDeterminationPresentationStatus,
} from "./workflow-node-diagnostics.ts";

interface TaskDeterminationCopy {
  status: TaskDeterminationPresentationStatus;
  label: string;
  messages: string[];
}

function taskDeterminationCopy(workflowTask: TimelineWorkflowTask | null): TaskDeterminationCopy {
  const messages = taskDeterminationMessages(workflowTask);
  if (messages.length > 0) {
    const status = taskDeterminationStatus(workflowTask);
    return {
      status: status === "ready" ? "diagnostic" : status,
      label: workflowTask
        ? status === "incomplete"
          ? "Undetermined"
          : status === "diagnostic"
            ? "Placement diagnostic"
            : "Fully determined with diagnostics"
        : "Needs backend determination data",
      messages,
    };
  }

  return {
    status: "ready",
    label: "Fully determined",
    messages: ["Backend workflow data fully determines this task's event-axis placement."],
  };
}

export function TaskPlacementWarning(props: {
  workflowTask: TimelineWorkflowTask | null;
}) {
  const determination = taskDeterminationCopy(props.workflowTask);
  if (determination.status === "ready") {
    return null;
  }

  return (
    <Stack
      className="task-placement-warning"
      data-placement-status={determination.status}
      data-testid="task-placement-warning"
      gap={4}
    >
      <Text fw={650} size="sm">
        {determination.label}
      </Text>
      <List spacing={2} size="sm">
        {determination.messages.map((message) => (
          <List.Item key={message}>{message}</List.Item>
        ))}
      </List>
    </Stack>
  );
}
