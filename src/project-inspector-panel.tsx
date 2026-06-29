import { useEffect, useRef, useState } from "react";
import { Button, Group, Text, Textarea } from "@mantine/core";

import { InspectorSurface } from "./inspector-components.tsx";
import { basenameFromPath } from "./project-tree-dom.ts";
import type { OpenProjectResult, ProjectReadmeEdit } from "./shell-types.ts";

interface ProjectReadmeDraft {
  readmeContent: string;
}

function projectReadmeDraftFromProject(project: OpenProjectResult): ProjectReadmeDraft {
  return {
    readmeContent: project.projectReadmeContent,
  };
}

function projectReadmeEditFromDraft(
  project: OpenProjectResult,
  draft: ProjectReadmeDraft,
): ProjectReadmeEdit {
  return {
    readmeContent: draft.readmeContent,
    expectedReadmeVersion: project.projectReadmeVersion,
  };
}

export interface ProjectInspectorPanelProps {
  project: OpenProjectResult;
  onEditProjectReadme: (edit: ProjectReadmeEdit) => Promise<void> | void;
}

export function ProjectInspectorPanel({
  project,
  onEditProjectReadme,
}: ProjectInspectorPanelProps) {
  const projectName = project.projectRoot ? basenameFromPath(project.projectRoot) : "Project";
  const [draft, setDraft] = useState(() => projectReadmeDraftFromProject(project));
  const acceptedDraftRef = useRef<ProjectReadmeDraft>(projectReadmeDraftFromProject(project));
  const [isSaving, setIsSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    const nextDraft = projectReadmeDraftFromProject(project);
    acceptedDraftRef.current = nextDraft;
    setDraft(nextDraft);
    setSaveError(null);
    setIsSaving(false);
  }, [project.projectRoot, project.projectReadmeContent, project.projectReadmeVersion]);

  const persistDraft = async (nextDraft: ProjectReadmeDraft) => {
    setDraft(nextDraft);
    setIsSaving(true);
    setSaveError(null);
    try {
      await onEditProjectReadme(projectReadmeEditFromDraft(project, nextDraft));
      acceptedDraftRef.current = nextDraft;
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
      setDraft(nextDraft);
    } finally {
      setIsSaving(false);
    }
  };

  const revertDraft = () => {
    setDraft(acceptedDraftRef.current);
    setSaveError(null);
    setIsSaving(false);
  };

  return (
    <InspectorSurface
      aria-label="Project inspector"
      className="project-inspector"
      contentClassName="project-inspector-layout"
      testId="project-inspector"
    >
      <Text className="inspector-title" component="h2" fw={700} size="xl">
        {projectName}
      </Text>
      <div className="project-readme-editor" data-testid="project-readme-section">
        <Textarea
          id="edit-project-readme"
          data-testid="edit-project-readme"
          name="project-readme"
          label="README"
          classNames={{
            input: "project-readme-control-input",
            root: "project-readme-control",
            wrapper: "project-readme-control-wrapper",
          }}
          minRows={8}
          spellCheck={false}
          aria-label="Project README"
          aria-describedby={saveError || isSaving ? "project-readme-save-state" : undefined}
          value={draft.readmeContent}
          onInput={(event) => {
            setDraft({ readmeContent: event.currentTarget.value });
          }}
          onBlur={(event) => {
            const nextDraft = { readmeContent: event.currentTarget.value };
            void persistDraft(nextDraft);
          }}
        />
        {(saveError || isSaving) && (
          <Text
            c={saveError ? "red" : "dimmed"}
            id="project-readme-save-state"
            size="sm"
            role={saveError ? "alert" : "status"}
          >
            {saveError ?? "Saving project README..."}
          </Text>
        )}
        {saveError && (
          <Group gap="xs">
            <Button
              aria-label="Retry project README save"
              loading={isSaving}
              size="xs"
              variant="light"
              onClick={() => {
                void persistDraft(draft);
              }}
            >
              Retry project README save
            </Button>
            <Button
              aria-label="Revert project README changes"
              disabled={isSaving}
              size="xs"
              variant="subtle"
              onClick={revertDraft}
            >
              Revert project README changes
            </Button>
          </Group>
        )}
      </div>
    </InspectorSurface>
  );
}
