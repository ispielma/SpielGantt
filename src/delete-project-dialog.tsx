import type { FormEvent } from "react";

import { Button, Code, Group, Modal, Stack, Text, TextInput } from "@mantine/core";

import { modalTitle } from "./dialog-primitives.tsx";

export interface DeleteProjectDialogState {
  open: boolean;
  projectPath: string;
  projectName: string;
  confirmationText: string;
  confirmationPhrase: string;
  errorMessage: string | null;
  submitting: boolean;
}

export const emptyDeleteProjectDialogState: DeleteProjectDialogState = {
  open: false,
  projectPath: "",
  projectName: "",
  confirmationText: "",
  confirmationPhrase: "",
  errorMessage: null,
  submitting: false,
};

interface DeleteProjectDialogProps {
  state: DeleteProjectDialogState;
  onConfirmationTextChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: () => void;
}

export function deleteProjectConfirmationPhrase(projectName: string): string {
  return `DELETE ${projectName}`;
}

export function DeleteProjectDialog({
  state,
  onConfirmationTextChange,
  onCancel,
  onSubmit,
}: DeleteProjectDialogProps) {
  if (!state.open) {
    return null;
  }

  const confirmed = state.confirmationText === state.confirmationPhrase;
  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (confirmed && !state.submitting) {
      onSubmit();
    }
  };

  return (
    <div
      aria-label="Delete Project"
      data-testid="delete-project-dialog-overlay"
      role="dialog"
    >
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        title={modalTitle("Delete Project", "delete-project-dialog-title")}
      >
        <form
          aria-describedby={state.errorMessage ? "delete-project-error" : undefined}
          className="delete-project-form"
          data-testid="delete-project-form"
          onSubmit={handleSubmit}
        >
          <Stack gap="md">
            <Text>{`Delete project '${state.projectName}'?`}</Text>
            <Text size="sm">
              This will delete the entire project folder and all contents inside it, including
              user files that are not owned by SpielGantt.
            </Text>
            <Text size="sm">
              Project folder: <Code>{state.projectPath}</Code>
            </Text>
            <Text size="sm">
              Type <Code>{state.confirmationPhrase}</Code> to confirm.
            </Text>
            <TextInput
              id="delete-project-confirmation"
              data-testid="delete-project-confirmation"
              name="delete-project-confirmation"
              autoComplete="off"
              defaultValue={state.confirmationText}
              label="Confirmation phrase"
              aria-label="Confirmation phrase"
              onChange={(event) => onConfirmationTextChange(event.currentTarget.value)}
              onInput={(event) => onConfirmationTextChange(event.currentTarget.value)}
            />
            {state.errorMessage ? (
              <Text
                c="red"
                id="delete-project-error"
                className="dialog-error"
                data-testid="delete-project-error"
                aria-live="polite"
                role="alert"
                size="sm"
              >
                {state.errorMessage}
              </Text>
            ) : null}
            <Group justify="flex-end">
              <Button
                data-testid="cancel-delete-project"
                type="button"
                variant="default"
                onClick={onCancel}
                disabled={state.submitting}
              >
                Cancel
              </Button>
              <Button
                data-testid="confirm-delete-project"
                type="submit"
                color="red"
                disabled={!confirmed || state.submitting}
              >
                {state.submitting ? "Deleting project..." : "Delete Project"}
              </Button>
            </Group>
          </Stack>
        </form>
      </Modal>
    </div>
  );
}
