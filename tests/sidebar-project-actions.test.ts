import assert from "node:assert/strict";
import test from "node:test";

import type { Window } from "happy-dom";

import {
  clickTreeItem,
  deferred,
  getByRole,
  InMemoryRememberedProjectsSettings,
  projectFixture,
  rememberedProjectRecord,
  setNativeValue,
  taskFixture,
  waitForHandlers,
  withRenderedShell,
} from "./support/frontend-shell.ts";

function submitDialog(root: HTMLElement, window: Window, buttonName: RegExp): void {
  const button = getByRole(root, "button", buttonName) as HTMLButtonElement;
  const form = button.form;
  if (!form) {
    throw new Error(`expected ${buttonName} button to submit a form`);
  }

  form.dispatchEvent(new window.Event("submit", { bubbles: true, cancelable: true }));
}

function queryByRole(root: Element, role: Parameters<typeof getByRole>[1], name: RegExp) {
  try {
    return getByRole(root, role, name);
  } catch {
    return null;
  }
}

function assertActiveElement(
  root: HTMLElement,
  expected: HTMLElement,
  message: string,
): void {
  assert.ok(root.ownerDocument.activeElement === expected, message);
}

test("project creation from the sidebar opens and remembers the new project", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings();
  const createdProjects: Array<{ projectName: string; parentDestination: string }> = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        createProjectAction: async (projectName, parentDestination) => {
          createdProjects.push({ projectName, parentDestination });
          return projectFixture({
            selectedPath: `${parentDestination}/${projectName}`,
            projectRoot: `${parentDestination}/${projectName}`,
          });
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt"
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      assertActiveElement(
        root,
        projectNameInput,
        "new project dialog should focus the project name input",
      );
      projectNameInput.value = "Experiment Plan";
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Project$/);
      await waitForHandlers();

      assert.deepEqual(createdProjects, [
        {
          projectName: "Experiment Plan",
          parentDestination: "/tmp/fixture-spielgantt",
        },
      ]);
      assert.equal(
        getByRole(root, "treeitem", /^Experiment Plan$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal((await rememberedProjects.listProjects()).length, 1);
    },
  );
});

test("New Project dialog shows the default destination and updates the final folder path", async () => {
  await withRenderedShell(
    {
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt"
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      const destination = getByRole(root, "group", /^New project destination$/);
      assert.match(destination.textContent ?? "", /Parent destination/);
      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt/);
      assert.match(destination.textContent ?? "", /Final project folder/);
      assert.match(destination.textContent ?? "", /Enter a project name/i);

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();

      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt\/Experiment Plan/);

      setNativeValue(projectNameInput, "Follow Up Run");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();

      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt\/Follow Up Run/);
    },
  );
});

test("New Project dialog reports when no default parent destination can be resolved", async () => {
  const createAttempts: Array<{ projectName: string; parentDestination: string }> = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        createProjectAction: async (projectName, parentDestination) => {
          createAttempts.push({ projectName, parentDestination });
          return projectFixture({
            selectedPath: `${parentDestination}/${projectName}`,
            projectRoot: `${parentDestination}/${projectName}`,
          });
        }
      },
      desktop: {
        resolveDefaultProjectParent: async () => null
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.match(
        getByRole(root, "alert", /Could not determine the default project destination/i)
          .textContent ?? "",
        /choose a parent destination/i,
      );

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Project$/);
      await waitForHandlers();

      assert.deepEqual(createAttempts, []);
      assert.match(root.textContent ?? "", /Choose a parent destination before creating/i);
    },
  );
});

test("New Project dialog chooses a parent destination for project creation", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings();
  const createdProjects: Array<{ projectName: string; parentDestination: string }> = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        createProjectAction: async (projectName, parentDestination) => {
          createdProjects.push({ projectName, parentDestination });
          return projectFixture({
            selectedPath: "/tmp/custom-spielgantt-projects/Experiment Plan",
            projectRoot: "/tmp/custom-spielgantt-projects/Experiment Plan",
          });
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt",
        pickProjectParentDestination: async () => "/tmp/custom-spielgantt-projects"
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();

      const destination = getByRole(root, "group", /^New project destination$/);
      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt\/Experiment Plan/);

      getByRole(root, "button", /^Choose parent destination$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.match(destination.textContent ?? "", /Parent destination/);
      assert.match(destination.textContent ?? "", /\/tmp\/custom-spielgantt-projects/);
      assert.match(
        destination.textContent ?? "",
        /\/tmp\/custom-spielgantt-projects\/Experiment Plan/,
      );

      submitDialog(root, window, /^Create Project$/);
      await waitForHandlers();

      assert.deepEqual(createdProjects, [
        {
          projectName: "Experiment Plan",
          parentDestination: "/tmp/custom-spielgantt-projects",
        },
      ]);
      assert.equal(
        getByRole(root, "treeitem", /^Experiment Plan$/).getAttribute("aria-selected"),
        "true",
      );
      assert.deepEqual(
        (await rememberedProjects.listProjects()).map((project) => project.projectPath),
        ["/tmp/custom-spielgantt-projects/Experiment Plan"],
      );
    },
  );
});

test("New Project dialog keeps the current destination when folder picking is cancelled", async () => {
  await withRenderedShell(
    {
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt",
        pickProjectParentDestination: async () => null
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();

      const destination = getByRole(root, "group", /^New project destination$/);
      const pathBeforeCancel = destination.textContent;

      getByRole(root, "button", /^Choose parent destination$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(destination.textContent, pathBeforeCancel);
      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt/);
      assert.match(destination.textContent ?? "", /\/tmp\/fixture-spielgantt\/Experiment Plan/);
    },
  );
});

test("New Project dialog keeps a chosen destination when the default destination resolves later", async () => {
  const defaultDestination = deferred<string>();

  await withRenderedShell(
    {
      desktop: {
        resolveDefaultProjectParent: async () => defaultDestination.promise,
        pickProjectParentDestination: async () => "/tmp/custom-spielgantt-projects"
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();

      const destination = getByRole(root, "group", /^New project destination$/);
      getByRole(root, "button", /^Choose parent destination$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      defaultDestination.resolve("/tmp/fixture-spielgantt");
      await waitForHandlers();
      await waitForHandlers();

      assert.match(destination.textContent ?? "", /\/tmp\/custom-spielgantt-projects/);
      assert.match(
        destination.textContent ?? "",
        /\/tmp\/custom-spielgantt-projects\/Experiment Plan/,
      );
      assert.doesNotMatch(destination.textContent ?? "", /\/tmp\/fixture-spielgantt/);
    },
  );
});

test("New Project dialog surfaces creation validation without opening a partial project", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings();
  const attemptedProjects: string[] = [];
  const openedProjectRoots: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        createProjectAction: async (projectName) => {
          attemptedProjects.push(projectName);
          throw new Error(
            "project directory '/tmp/fixture-spielgantt/Experiment Plan' already exists",
          );
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt"
      },
      projectWatch: {
        subscribeProjectSessionChanges: async (projectRoot) => {
          openedProjectRoots.push(projectRoot);
          return {
            status: { watching: false, message: "watch unavailable" },
            unsubscribe: () => {},
          };
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      setNativeValue(projectNameInput, "Experiment Plan");
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Project$/);
      await waitForHandlers();

      assert.deepEqual(attemptedProjects, ["Experiment Plan"]);
      assert.match(root.textContent ?? "", /already exists/);
      assert.deepEqual(openedProjectRoots, []);
      assert.equal((await rememberedProjects.listProjects()).length, 0);
    },
  );
});

test("Projects header exposes a compact Add project action for onboarding flows", async () => {
  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => new InMemoryRememberedProjectsSettings()
      }
    },
    async ({ root, window }) => {
      const addProjectButton = getByRole(root, "button", /^Add project$/);
      assert.equal(addProjectButton.getAttribute("aria-haspopup"), "menu");

      addProjectButton.dispatchEvent(new window.Event("click", { bubbles: true }));
      await waitForHandlers();

      assert.ok(getByRole(root, "menuitem", /^Start from scratch$/));
      assert.ok(getByRole(root, "menuitem", /^Use an existing folder$/));
    },
  );
});

test("Projects actions onboard an existing folder from accessible menu items and remember it", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings();
  const onboardedPaths: string[] = [];
  const openedPaths: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) => {
          openedPaths.push(path);
          throw new Error("Use an existing folder should onboard before opening");
        },
        onboardProjectAction: async (path) => {
          onboardedPaths.push(path);
          return projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/workflow",
            projectRoot: "/tmp/fixture-spielgantt/workflow",
          });
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Use an existing folder$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(onboardedPaths, ["/tmp/fixture-spielgantt/workflow"]);
      assert.deepEqual(openedPaths, []);
      assert.equal(
        getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal((await rememberedProjects.listProjects()).length, 1);
    },
  );
});

test("Start from scratch still creates a new project instead of onboarding a folder", async () => {
  const createdProjects: string[] = [];
  const onboardedPaths: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        createProjectAction: async (projectName) => {
          createdProjects.push(projectName);
          return projectFixture({
            selectedPath: `/tmp/fixture-spielgantt/${projectName}`,
            projectRoot: `/tmp/fixture-spielgantt/${projectName}`,
          });
        },
        onboardProjectAction: async (path) => {
          onboardedPaths.push(path);
          return projectFixture({
            selectedPath: path,
            projectRoot: path,
          });
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();

      const projectNameInput = getByRole(root, "textbox", /^Project name$/) as HTMLInputElement;
      projectNameInput.value = "Scratch Project";
      projectNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Project$/);
      await waitForHandlers();

      assert.deepEqual(createdProjects, ["Scratch Project"]);
      assert.deepEqual(onboardedPaths, []);
      assert.equal(
        getByRole(root, "treeitem", /^Scratch Project$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("Delete Project confirmation names the folder and requires the exact phrase", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/Workflow Project"),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/Workflow Project",
            projectRoot: "/tmp/fixture-spielgantt/Workflow Project",
          }),
        onboardProjectAction: async () =>
          projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/Workflow Project",
            projectRoot: "/tmp/fixture-spielgantt/Workflow Project",
          })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Project actions for Workflow Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "menuitem", /^Delete project Workflow Project$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const dialog = getByRole(root, "dialog", /^Delete Project$/);
      assert.match(dialog.textContent ?? "", /Workflow Project/);
      assert.match(dialog.textContent ?? "", /\/tmp\/fixture-spielgantt\/Workflow Project/);
      assert.match(dialog.textContent ?? "", /entire project folder and all contents inside it/i);
      assert.match(dialog.textContent ?? "", /including user files/i);

      const deleteButton = getByRole(root, "button", /^Delete Project$/) as HTMLButtonElement;
      assert.equal(deleteButton.disabled, true);

      const input = getByRole(root, "textbox", /^Confirmation phrase$/) as HTMLInputElement;
      assertActiveElement(
        root,
        input,
        "delete project dialog should focus the confirmation input",
      );
      setNativeValue(input, "delete Workflow Project");
      input.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();
      assert.equal(
        (getByRole(root, "button", /^Delete Project$/) as HTMLButtonElement).disabled,
        true,
      );

      setNativeValue(input, "DELETE Workflow Project");
      input.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();
      assert.equal(
        (getByRole(root, "button", /^Delete Project$/) as HTMLButtonElement).disabled,
        false,
      );

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.throws(() => getByRole(root, "dialog", /^Delete Project$/));
      assert.equal((await rememberedProjects.listProjects()).length, 1);
      assert.ok(getByRole(root, "treeitem", /^Workflow Project$/));
    },
  );
});

test("confirmed Delete Project removes the matching remembered project and clears the active workspace", async () => {
  const activeProjectPath = "/tmp/fixture-spielgantt/Active Project";
  const archiveProjectPath = "/tmp/fixture-spielgantt/Archive Project";
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(activeProjectPath),
    rememberedProjectRecord(archiveProjectPath),
  ]);
  const trashedProjectPaths: string[] = [];
  let unsubscribeCount = 0;

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          }),
        onboardProjectAction: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        trashProjectFolderAction: async (projectPath) => {
          trashedProjectPaths.push(projectPath);
          return {
            status: "moved-to-trash",
            projectPath,
            mechanism: "finder-trash",
          };
        }
      },
      projectWatch: {
        subscribeProjectSessionChanges: async () => ({
          status: { watching: true, message: "watching" },
          unsubscribe: () => {
            unsubscribeCount += 1;
          },
        })
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^Active Project$/);
      await waitForHandlers();
      await waitForHandlers();
      assert.ok(getByRole(root, "treeitem", /^analysis$/));

      getByRole(root, "button", /^Project actions for Active Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Delete project Active Project$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const confirmation = getByRole(
        root,
        "textbox",
        /^Confirmation phrase$/,
      ) as HTMLInputElement;
      setNativeValue(confirmation, "DELETE Active Project");
      confirmation.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();
      submitDialog(root, window, /^Delete Project$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(trashedProjectPaths, [activeProjectPath]);
      assert.deepEqual(
        (await rememberedProjects.listProjects()).map((record) => record.projectPath),
        [archiveProjectPath],
      );
      assert.throws(() => getByRole(root, "treeitem", /^Active Project$/));
      assert.ok(getByRole(root, "treeitem", /^Archive Project$/));
      assert.match(root.textContent ?? "", /Choose a SpielGantt project folder to begin/);
      assert.equal(unsubscribeCount, 1);
    },
  );
});

test("failed Delete Project leaves the remembered project and active workspace intact", async () => {
  const projectPath = "/tmp/fixture-spielgantt/Failure Project";
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(projectPath),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          }),
        onboardProjectAction: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        trashProjectFolderAction: async () => ({
          status: "failed",
          projectPath,
          message: "Trash permission denied",
        })
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^Failure Project$/);
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "button", /^Project actions for Failure Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Delete project Failure Project$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const confirmation = getByRole(
        root,
        "textbox",
        /^Confirmation phrase$/,
      ) as HTMLInputElement;
      setNativeValue(confirmation, "DELETE Failure Project");
      confirmation.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();
      submitDialog(root, window, /^Delete Project$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.match(getByRole(root, "dialog", /^Delete Project$/).textContent ?? "", /Trash permission denied/);
      assert.deepEqual(
        (await rememberedProjects.listProjects()).map((record) => record.projectPath),
        [projectPath],
      );
      assert.ok(getByRole(root, "treeitem", /^Failure Project$/));
      assert.ok(getByRole(root, "treeitem", /^analysis$/));
    },
  );
});

test("thrown Delete Project trash errors leave the dialog recoverable", async () => {
  const projectPath = "/tmp/fixture-spielgantt/Thrown Failure Project";
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(projectPath),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          }),
        onboardProjectAction: async (path) =>
          projectFixture({
            selectedPath: path,
            projectRoot: path,
            tasks: [taskFixture("analysis", { path: `${path}/analysis` })],
          })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        trashProjectFolderAction: async () => {
          throw new Error("Native trash bridge crashed");
        }
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^Thrown Failure Project$/);
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "button", /^Project actions for Thrown Failure Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Delete project Thrown Failure Project$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const confirmation = getByRole(
        root,
        "textbox",
        /^Confirmation phrase$/,
      ) as HTMLInputElement;
      setNativeValue(confirmation, "DELETE Thrown Failure Project");
      confirmation.dispatchEvent(new window.Event("input", { bubbles: true }));
      await waitForHandlers();
      submitDialog(root, window, /^Delete Project$/);
      await waitForHandlers();
      await waitForHandlers();

      const dialog = getByRole(root, "dialog", /^Delete Project$/);
      assert.match(dialog.textContent ?? "", /Native trash bridge crashed/);
      assert.equal(
        (getByRole(root, "button", /^Delete Project$/) as HTMLButtonElement).disabled,
        false,
        "the delete button should be usable after a thrown trash failure",
      );
      assert.deepEqual(
        (await rememberedProjects.listProjects()).map((record) => record.projectPath),
        [projectPath],
      );
      assert.ok(getByRole(root, "treeitem", /^Thrown Failure Project$/));
      assert.ok(getByRole(root, "treeitem", /^analysis$/));
    },
  );
});

test("Remove from Sidebar does not call the project trash capability", async () => {
  const projectPath = "/tmp/fixture-spielgantt/Sidebar Only Project";
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(projectPath),
  ]);
  const trashCalls: string[] = [];

  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        trashProjectFolderAction: async (path) => {
          trashCalls.push(path);
          return { status: "failed", projectPath: path, message: "unexpected trash call" };
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Project actions for Sidebar Only Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "menuitem", /^Remove project Sidebar Only Project from sidebar$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.deepEqual(trashCalls, []);
      assert.deepEqual(await rememberedProjects.listProjects(), []);
      assert.throws(() => getByRole(root, "treeitem", /^Sidebar Only Project$/));
    },
  );
});

test("Remove from Sidebar clears the active project when it is the only open project", async () => {
  const projectPath = "/tmp/fixture-spielgantt/Active Sidebar Only Project";
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(projectPath),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          projectFixture({
            selectedPath: projectPath,
            projectRoot: projectPath,
            tasks: [
              taskFixture("analysis", {
                path: `${projectPath}/analysis`,
              }),
            ],
          }),
        onboardProjectAction: async () =>
          projectFixture({
            selectedPath: projectPath,
            projectRoot: projectPath,
            tasks: [
              taskFixture("analysis", {
                path: `${projectPath}/analysis`,
              }),
            ],
          })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^Active Sidebar Only Project$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^analysis$/));

      getByRole(root, "button", /^Project actions for Active Sidebar Only Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "menuitem", /^Remove project Active Sidebar Only Project from sidebar$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(await rememberedProjects.listProjects(), []);
      assert.throws(() => getByRole(root, "treeitem", /^Active Sidebar Only Project$/));
      assert.throws(() => getByRole(root, "treeitem", /^analysis$/));
      assert.ok(queryByRole(root, "button", /^Add project$/));
    },
  );
});

test("missing remembered projects do not offer Delete Project", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/Missing Project", [], null, true),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => {
          throw new Error("project folder is gone");
        },
        onboardProjectAction: async () => {
          throw new Error("project folder is gone");
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "button", /^Project actions for Missing Project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "menuitem", /^Delete project Missing Project$/i), null);
      assert.ok(getByRole(root, "menuitem", /^Find project Missing Project$/i));
      assert.ok(getByRole(root, "menuitem", /^Remove project Missing Project from sidebar$/i));
    },
  );
});
