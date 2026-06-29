import { Button, Group, Modal, Stack, Text } from "@mantine/core";

import type { AdoptableTaskFolder } from "./shell-types.ts";

export interface TaskFromFolderDialogState {
  open: boolean;
  projectPath: string;
  candidates: AdoptableTaskFolder[];
  selectedFolderPath: string;
  errorMessage: string | null;
  submitting: boolean;
}

export const emptyTaskFromFolderDialogState: TaskFromFolderDialogState = {
  open: false,
  projectPath: "",
  candidates: [],
  selectedFolderPath: "",
  errorMessage: null,
  submitting: false,
};

interface TaskFromFolderDialogProps {
  state: TaskFromFolderDialogState;
  onSelect: (folderPath: string) => void;
  onCancel: () => void;
  onSubmit: () => void;
}

export function TaskFromFolderDialog({
  state,
  onSelect,
  onCancel,
  onSubmit,
}: TaskFromFolderDialogProps) {
  if (!state.open) {
    return null;
  }

  const selected = state.candidates.find(
    (candidate) => candidate.folderPath === state.selectedFolderPath,
  );

  return (
    <div data-testid="task-from-folder-dialog-overlay">
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        title={
          <Text component="span" id="task-from-folder-dialog-title" size="xl" fw={650}>
            Create Task from Folder
          </Text>
        }
      >
        <Stack gap="md">
          <section aria-label="Adoptable task folders">
            <Stack gap="xs">
              {state.candidates.length ? (
                state.candidates.map((candidate) => (
                  <Button
                    key={candidate.folderPath}
                    aria-pressed={candidate.folderPath === state.selectedFolderPath}
                    data-testid="task-from-folder-candidate"
                    justify="flex-start"
                    type="button"
                    variant={
                      candidate.folderPath === state.selectedFolderPath ? "filled" : "default"
                    }
                    onClick={() => onSelect(candidate.folderPath)}
                  >
                    {candidate.projectRelativePath}
                  </Button>
                ))
              ) : (
                <Text size="sm">No adoptable folders found.</Text>
              )}
            </Stack>
          </section>
          {state.errorMessage ? (
            <Text
              c="red"
              id="task-from-folder-error"
              className="dialog-error"
              data-testid="task-from-folder-error"
              aria-live="polite"
              role="alert"
              size="sm"
            >
              {state.errorMessage}
            </Text>
          ) : null}
          <Group justify="flex-end">
            <Button
              data-testid="cancel-task-from-folder"
              type="button"
              variant="default"
              onClick={onCancel}
            >
              Cancel
            </Button>
            <Button
              data-testid="confirm-task-from-folder"
              type="button"
              disabled={!selected || state.submitting}
              onClick={onSubmit}
            >
              {state.submitting ? "Adopting..." : "OK"}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
