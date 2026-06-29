import type { FormEvent } from "react";

import { Button, Group, Modal, Stack, Text, TextInput } from "@mantine/core";
import { IconFolderOpen } from "@tabler/icons-react";

import { modalTitle } from "./dialog-primitives.tsx";
import { projectFolderPreview } from "./project-destination-paths.ts";
import type { NewProjectDialogState } from "./shell-dialogs.tsx";

interface NewProjectDialogProps {
  state: NewProjectDialogState;
  onProjectNameChange: (projectName: string) => void;
  onChooseParentDestination: () => void;
  onCancel: () => void;
  onSubmit: (projectName: string) => void;
}

export function NewProjectDialog({
  state,
  onProjectNameChange,
  onChooseParentDestination,
  onCancel,
  onSubmit,
}: NewProjectDialogProps) {
  if (!state.open) {
    return null;
  }

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const FormDataCtor = event.currentTarget.ownerDocument.defaultView?.FormData ?? FormData;
    const formData = new FormDataCtor(event.currentTarget);
    onSubmit(String(formData.get("project-name") ?? "").trim());
  };

  return (
    <div data-testid="new-project-dialog-overlay">
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        title={modalTitle("New Project", "new-project-dialog-title")}
      >
        <form
          aria-describedby={state.errorMessage ? "new-project-error" : undefined}
          className="new-project-form"
          data-testid="new-project-form"
          onSubmit={handleSubmit}
        >
          <Stack gap="md">
            <TextInput
              id="new-project-name"
              data-testid="new-project-name"
              name="project-name"
              autoComplete="off"
              defaultValue={state.projectName}
              label="Project name"
              aria-label="Project name"
              onInput={(event) => onProjectNameChange(event.currentTarget.value)}
            />
            <div
              aria-label="New project destination"
              className="new-project-destination"
              role="group"
            >
              <div className="new-project-destination-row">
                <Group align="center" justify="space-between" gap="sm" wrap="nowrap">
                  <Text className="new-project-destination-label" size="sm">
                    Parent destination
                  </Text>
                  <Button
                    aria-label="Choose parent destination"
                    className="new-project-destination-picker"
                    data-testid="choose-new-project-parent"
                    disabled={state.choosingParentDestination}
                    leftSection={<IconFolderOpen aria-hidden="true" focusable={false} size={16} />}
                    onClick={onChooseParentDestination}
                    size="compact-sm"
                    type="button"
                    variant="default"
                  >
                    Choose...
                  </Button>
                </Group>
                <Text className="new-project-destination-path" component="code" size="sm">
                  {state.parentDestination ?? "Resolving destination..."}
                </Text>
              </div>
              <div className="new-project-destination-row">
                <Text className="new-project-destination-label" size="sm">
                  Final project folder
                </Text>
                <Text className="new-project-destination-path" component="code" size="sm">
                  {projectFolderPreview(state.parentDestination, state.projectName)}
                </Text>
              </div>
            </div>
            {state.errorMessage ? (
              <Text
                c="red"
                id="new-project-error"
                className="dialog-error"
                data-testid="new-project-error"
                aria-live="polite"
                role="alert"
                size="sm"
              >
                {state.errorMessage}
              </Text>
            ) : null}
            <Group justify="flex-end">
              <Button
                data-testid="cancel-new-project"
                type="button"
                variant="default"
                onClick={onCancel}
              >
                Cancel
              </Button>
              <Button data-testid="create-new-project" type="submit" disabled={state.submitting}>
                {state.submitting ? "Creating project..." : "Create Project"}
              </Button>
            </Group>
          </Stack>
        </form>
      </Modal>
    </div>
  );
}
