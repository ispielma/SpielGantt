import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { InMemoryRememberedProjectsSettings, type RememberedProjectRecord, type RememberedProjectsSettings } from "./remembered-projects.ts";
import { replaceRememberedProjectRecord } from "./shell-state.ts";
import type { ProjectSessionPersistenceCapabilities } from "./shell-types.ts";

export interface RememberedProjectsStateController {
  ready: boolean;
  rememberedProjects: RememberedProjectRecord[];
  activeProjectPath: string | null;
  setActiveProjectPath(projectPath: string | null): void;
  upsertProjectRecord(record: RememberedProjectRecord): Promise<void>;
  removeProject(projectPath: string): Promise<void>;
}

export function useRememberedProjectsState(
  loadRememberedProjectsSettings: ProjectSessionPersistenceCapabilities["loadRememberedProjectsSettings"],
): RememberedProjectsStateController {
  const [ready, setReady] = useState(false);
  const [rememberedProjects, setRememberedProjects] = useState<RememberedProjectRecord[]>([]);
  const [activeProjectPath, setActiveProjectPath] = useState<string | null>(null);
  const settingsRef = useRef<RememberedProjectsSettings>(
    new InMemoryRememberedProjectsSettings(),
  );

  useEffect(() => {
    let cancelled = false;

    void Promise.resolve(loadRememberedProjectsSettings())
      .then(async (settings) => {
        settingsRef.current = settings;
        const loadedProjects = await settings.listProjects();
        if (cancelled) {
          return;
        }

        setRememberedProjects(loadedProjects);
        setReady(true);
      })
      .catch(() => {
        if (cancelled) {
          return;
        }

        settingsRef.current = new InMemoryRememberedProjectsSettings();
        setRememberedProjects([]);
        setReady(true);
      });

    return () => {
      cancelled = true;
    };
  }, [loadRememberedProjectsSettings]);

  const upsertProjectRecord = useCallback(async (record: RememberedProjectRecord) => {
    setRememberedProjects((currentProjects) =>
      replaceRememberedProjectRecord(currentProjects, record),
    );
    await settingsRef.current.upsertProjectRecord(record);
  }, []);

  const removeProject = useCallback(async (projectPath: string) => {
    setRememberedProjects((currentProjects) =>
      currentProjects.filter((record) => record.projectPath !== projectPath),
    );
    setActiveProjectPath((currentProjectPath) =>
      currentProjectPath === projectPath ? null : currentProjectPath,
    );
    await settingsRef.current.removeProject(projectPath);
  }, []);

  return useMemo(
    () => ({
      ready,
      rememberedProjects,
      activeProjectPath,
      setActiveProjectPath,
      upsertProjectRecord,
      removeProject,
    }),
    [
      activeProjectPath,
      ready,
      rememberedProjects,
      removeProject,
      upsertProjectRecord,
    ],
  );
}
