import { Button, Code, Group, Modal, Paper, Stack, Text } from "@mantine/core";

import { modalTitle } from "./dialog-primitives.tsx";
import type { TaskFolderAlignmentDialogState } from "./shell-dialogs.tsx";

export function TaskFolderAlignmentDialog({
  state,
  onCancel,
  onApply,
}: {
  state: TaskFolderAlignmentDialogState;
  onCancel: () => void;
  onApply: () => void;
}) {
  if (!state.open || !state.plan) {
    return null;
  }

  const issueSummary = state.plan.preflightIssues.length
    ? "Cannot apply until the listed folder collisions are resolved."
    : "Review the planned folder renames before applying them.";

  return (
    <div data-testid="alignment-dialog-overlay">
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        title={modalTitle("Confirm Folder Alignment", "alignment-dialog-title")}
      >
        <Stack gap="md">
          <Text size="sm">{issueSummary}</Text>
          <Paper withBorder radius="md" p="sm">
            <Stack gap="xs">
              <Text size="sm">
                <Text component="span" fw={600}>
                  Project:
                </Text>{" "}
                {state.projectName}
              </Text>
              <Text size="sm">
                <Text component="span" fw={600}>
                  Folder:
                </Text>{" "}
                <Code>{state.projectPath}</Code>
              </Text>
            </Stack>
          </Paper>
          <section aria-label="Planned renames">
            <Stack gap="xs">
              <Text component="h3" fw={600} size="sm">
                Planned renames
              </Text>
              {state.plan.operations.length > 0 ? (
                <ul className="alignment-plan-list" data-testid="alignment-plan-renames">
                  {state.plan.operations.map((operation) => {
                    const rename = operation.renameTaskFolder;
                    return (
                      <li key={`${rename.taskId}:${rename.from}:${rename.to}`}>
                        <Stack gap={4}>
                          <Text fw={500} size="sm">
                            {rename.taskId}
                          </Text>
                          <Group gap="xs" wrap="wrap">
                            <Code>{rename.from}</Code>
                            <Text aria-hidden="true" size="sm">
                              →
                            </Text>
                            <Code>{rename.to}</Code>
                          </Group>
                        </Stack>
                      </li>
                    );
                  })}
                </ul>
              ) : (
                <Text size="sm">No folder renames are required.</Text>
              )}
            </Stack>
          </section>
          {state.plan.preflightIssues.length > 0 ? (
            <section aria-label="Folder collisions">
              <Stack gap="xs">
                <Text component="h3" fw={600} size="sm">
                  Folder collisions
                </Text>
                <ul className="alignment-plan-list" data-testid="alignment-plan-issues">
                  {state.plan.preflightIssues.map((issue) => (
                    <li key={issue.targetAlreadyExists}>
                      <Text size="sm">
                        target folder <Code>{issue.targetAlreadyExists}</Code> already exists
                      </Text>
                    </li>
                  ))}
                </ul>
              </Stack>
            </section>
          ) : null}
          {state.errorMessage ? (
            <Text
              c="red"
              id="alignment-dialog-error"
              className="dialog-error"
              data-testid="alignment-dialog-error"
              aria-live="polite"
              role="alert"
              size="sm"
            >
              {state.errorMessage}
            </Text>
          ) : null}
          <Group justify="flex-end">
            <Button
              data-testid="cancel-alignment"
              type="button"
              variant="default"
              onClick={onCancel}
            >
              Cancel
            </Button>
            <Button
              data-testid="apply-alignment"
              type="button"
              disabled={state.submitting || state.plan.preflightIssues.length > 0}
              onClick={onApply}
            >
              {state.submitting ? "Applying Alignment..." : "Apply Alignment and Open"}
            </Button>
          </Group>
        </Stack>
      </Modal>
    </div>
  );
}
