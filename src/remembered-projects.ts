export interface RememberedProjectRecord {
  projectPath: string;
  expanded: boolean;
  lastOpenedAt: string;
  lastSelectedTaskName: string | null;
}

export interface RememberedProjectsSettings {
  listProjects(): Promise<RememberedProjectRecord[]>;
  upsertProjectRecord(record: RememberedProjectRecord): Promise<void>;
  removeProject(projectPath: string): Promise<void>;
}

const REMEMBERED_PROJECTS_STORAGE_KEY = "spielgantt.rememberedProjects.v1";

function cloneRecord(record: RememberedProjectRecord): RememberedProjectRecord {
  return {
    projectPath: record.projectPath,
    expanded: record.expanded,
    lastOpenedAt: record.lastOpenedAt,
    lastSelectedTaskName: record.lastSelectedTaskName,
  };
}

function isRememberedProjectRecord(value: unknown): value is RememberedProjectRecord {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<RememberedProjectRecord>;
  return (
    typeof candidate.projectPath === "string" &&
    typeof candidate.expanded === "boolean" &&
    typeof candidate.lastOpenedAt === "string" &&
    (candidate.lastSelectedTaskName === null ||
      typeof candidate.lastSelectedTaskName === "string")
  );
}

function parseStoredRecords(serializedRecords: string | null): RememberedProjectRecord[] {
  if (!serializedRecords) {
    return [];
  }

  try {
    const parsed = JSON.parse(serializedRecords);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed.filter(isRememberedProjectRecord).map(cloneRecord);
  } catch {
    return [];
  }
}

function cloneRecords(records: RememberedProjectRecord[]): RememberedProjectRecord[] {
  return records.map(cloneRecord);
}

function replaceRecord(
  records: RememberedProjectRecord[],
  record: RememberedProjectRecord,
): RememberedProjectRecord[] {
  const nextRecords = [...records];
  const existingIndex = nextRecords.findIndex(
    (candidate) => candidate.projectPath === record.projectPath,
  );

  if (existingIndex === -1) {
    nextRecords.push(cloneRecord(record));
    return nextRecords;
  }

  nextRecords[existingIndex] = cloneRecord(record);
  return nextRecords;
}

export class InMemoryRememberedProjectsSettings implements RememberedProjectsSettings {
  private readonly records: RememberedProjectRecord[];

  constructor(initialRecords: RememberedProjectRecord[] = []) {
    this.records = cloneRecords(initialRecords);
  }

  async listProjects(): Promise<RememberedProjectRecord[]> {
    return cloneRecords(this.records);
  }

  async upsertProjectRecord(record: RememberedProjectRecord): Promise<void> {
    const nextRecords = replaceRecord(this.records, record);
    this.records.splice(0, this.records.length, ...nextRecords);
  }

  async removeProject(projectPath: string): Promise<void> {
    const remainingRecords = this.records.filter(
      (record) => record.projectPath !== projectPath,
    );
    this.records.splice(0, this.records.length, ...remainingRecords);
  }
}

export class LocalStorageRememberedProjectsSettings implements RememberedProjectsSettings {
  constructor(
    private readonly storage: Storage,
    private readonly storageKey = REMEMBERED_PROJECTS_STORAGE_KEY,
  ) {}

  async listProjects(): Promise<RememberedProjectRecord[]> {
    return parseStoredRecords(this.storage.getItem(this.storageKey));
  }

  async upsertProjectRecord(record: RememberedProjectRecord): Promise<void> {
    const nextRecords = replaceRecord(await this.listProjects(), record);
    this.storage.setItem(this.storageKey, JSON.stringify(nextRecords));
  }

  async removeProject(projectPath: string): Promise<void> {
    const remainingRecords = (await this.listProjects()).filter(
      (record) => record.projectPath !== projectPath,
    );
    this.storage.setItem(this.storageKey, JSON.stringify(remainingRecords));
  }
}

export async function createProductionRememberedProjectsSettings(): Promise<RememberedProjectsSettings> {
  const storage = globalThis.window?.localStorage;

  return storage
    ? new LocalStorageRememberedProjectsSettings(storage)
    : new InMemoryRememberedProjectsSettings();
}

export function createRememberedProjectRecord(
  projectPath: string,
  selectedTaskName: string | null,
  expanded = false,
  lastOpenedAt = new Date().toISOString(),
): RememberedProjectRecord {
  return {
    projectPath,
    expanded,
    lastOpenedAt,
    lastSelectedTaskName: selectedTaskName,
  };
}

export async function rememberProjectReference(
  settings: RememberedProjectsSettings,
  projectPath: string,
  selectedTaskName: string | null,
  lastOpenedAt = new Date().toISOString(),
  expanded = false,
): Promise<void> {
  const currentRecords = await settings.listProjects();
  const currentRecord = currentRecords.find((record) => record.projectPath === projectPath);
  await settings.upsertProjectRecord({
    projectPath,
    expanded: currentRecord?.expanded ?? expanded,
    lastOpenedAt,
    lastSelectedTaskName: selectedTaskName,
  });
}

export async function forgetRememberedProject(
  settings: RememberedProjectsSettings,
  projectPath: string,
): Promise<void> {
  await settings.removeProject(projectPath);
}
