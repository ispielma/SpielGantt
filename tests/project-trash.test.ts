import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import test from "node:test";

import {
  moveProjectFolderToTrash,
  type ProjectTrashCommandRunner,
} from "../src/project-trash.ts";

async function pathExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function createProjectWithUserFile(name: string) {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-trash-test-"));
  const projectPath = join(root, name);
  await mkdir(join(projectPath, ".spielgantt"), { recursive: true });
  await writeFile(
    join(projectPath, ".spielgantt", "project.json"),
    JSON.stringify({ schema_version: 1, folder_naming: "task_id" }),
  );
  await writeFile(join(projectPath, "ordinary-user-file.txt"), "research notes\n");

  return { root, projectPath };
}

test("project trash wrapper moves the whole project folder with ordinary user files when the platform trash capability succeeds", async () => {
  const { root, projectPath } = await createProjectWithUserFile("Workflow Project");
  const fakeTrashRoot = join(root, "Fake Trash");
  await mkdir(fakeTrashRoot);

  const runner: ProjectTrashCommandRunner = {
    async execute(commandName, args) {
      assert.equal(commandName, "spielgantt-trash-macos");
      const pathArg = args.at(-1);
      assert.equal(pathArg, projectPath);
      await rename(projectPath, join(fakeTrashRoot, basename(projectPath)));
      return { code: 0, signal: null, stdout: "", stderr: "" };
    },
  };

  try {
    const result = await moveProjectFolderToTrash(projectPath, {
      detectPlatform: () => "macos",
      commandRunner: runner,
    });

    assert.equal(result.status, "moved-to-trash");
    assert.equal(await pathExists(projectPath), false);
    assert.equal(
      await readFile(
        join(fakeTrashRoot, "Workflow Project", "ordinary-user-file.txt"),
        "utf8",
      ),
      "research notes\n",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("project trash wrapper fails safely on unsupported platforms", async () => {
  const { root, projectPath } = await createProjectWithUserFile("Unsupported Workflow");
  const runner: ProjectTrashCommandRunner = {
    async execute() {
      throw new Error("unsupported platform must not invoke a trash command");
    },
  };

  try {
    const result = await moveProjectFolderToTrash(projectPath, {
      detectPlatform: () => "linux",
      commandRunner: runner,
    });

    assert.equal(result.status, "unsupported");
    assert.equal(await pathExists(projectPath), true);
    assert.equal(
      await readFile(join(projectPath, "ordinary-user-file.txt"), "utf8"),
      "research notes\n",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("project trash wrapper reports command failures without deleting the project folder", async () => {
  const { root, projectPath } = await createProjectWithUserFile("Failure Workflow");
  const runner: ProjectTrashCommandRunner = {
    async execute(commandName, args) {
      assert.equal(commandName, "spielgantt-trash-windows");
      assert.equal(args.at(-1), projectPath);
      return { code: 1, signal: null, stdout: "", stderr: "Recycle Bin is unavailable" };
    },
  };

  try {
    const result = await moveProjectFolderToTrash(projectPath, {
      detectPlatform: () => "windows",
      commandRunner: runner,
    });

    assert.deepEqual(result, {
      status: "failed",
      projectPath,
      message: "Recycle Bin is unavailable",
    });
    assert.equal(await pathExists(projectPath), true);
    assert.equal(
      await readFile(join(projectPath, "ordinary-user-file.txt"), "utf8"),
      "research notes\n",
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
