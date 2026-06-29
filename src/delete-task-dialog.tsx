import { Text } from "@mantine/core";

import { ConfirmationDialog } from "./dialog-primitives.tsx";
import type { TaskDeleteMode } from "./shell-types.ts";

export interface DeleteTaskDialogState {
  open: boolean;
  projectPath: string;
  taskId: string;
  taskName: string;
  errorMessage: string | null;
  submitting: TaskDeleteMode | null;
}

export const emptyDeleteTaskDialogState: DeleteTaskDialogState = {
  open: false,
  projectPath: "",
  taskId: "",
  taskName: "",
  errorMessage: null,
  submitting: null,
};

interface DeleteTaskDialogProps {
  state: DeleteTaskDialogState;
  onCancel: () => void;
  onSubmit: (mode: TaskDeleteMode) => void;
}

export function DeleteTaskDialog({ state, onCancel, onSubmit }: DeleteTaskDialogProps) {
  if (!state.open) {
    return null;
  }

  return (
    <ConfirmationDialog
      dialogTestId="delete-task-dialog-overlay"
      titleId="delete-task-dialog-title"
      title="Delete Task"
      errorId="delete-task-error"
      errorTestId="delete-task-error"
      errorMessage={state.errorMessage}
      message={`Delete task '${state.taskName}'?`}
      details={(
        <Text size="sm" c="dimmed">
          Remove from Chart keeps the task directory and user files. Delete Directory removes the
          whole task folder. Tasks that depend on this task inherit its blockers.
        </Text>
      )}
      cancelTestId="cancel-delete-task"
      cancelDisabled={state.submitting !== null}
      trapFocus={false}
      actions={[
        {
          testId: "remove-task-from-chart",
          label: "Remove from Chart",
          submittingLabel: "Removing...",
          isSubmitting: state.submitting === "remove-from-chart",
          disabled: state.submitting !== null,
          variant: "default",
          onClick: () => onSubmit("remove-from-chart"),
        },
        {
          testId: "delete-task-directory",
          label: "Delete Directory",
          submittingLabel: "Deleting...",
          isSubmitting: state.submitting === "delete-directory",
          disabled: state.submitting !== null,
          color: "red",
          onClick: () => onSubmit("delete-directory"),
        },
      ]}
      onCancel={onCancel}
    />
  );
}
