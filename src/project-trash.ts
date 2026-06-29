import { Command } from "@tauri-apps/plugin-shell";

export type ProjectTrashPlatform = "macos" | "windows" | "linux" | "unsupported";

export type ProjectTrashResult =
  | {
      status: "moved-to-trash";
      projectPath: string;
      mechanism: "finder-trash" | "windows-recycle-bin";
    }
  | {
      status: "unsupported";
      projectPath: string;
      message: string;
    }
  | {
      status: "failed";
      projectPath: string;
      message: string;
    };

export interface ProjectTrashCommandOutput {
  code: number | null;
  signal: number | null;
  stdout: string;
  stderr: string;
}

export interface ProjectTrashCommandRunner {
  execute(commandName: string, args: string[]): Promise<ProjectTrashCommandOutput>;
}

export interface ProjectTrashDeps {
  detectPlatform?: () => ProjectTrashPlatform;
  commandRunner?: ProjectTrashCommandRunner;
}

const macosFinderTrashScriptArgs = [
  "on run argv",
  "set targetPath to item 1 of argv",
  'tell application "Finder" to delete (POSIX file targetPath as alias)',
  "end run",
].flatMap((scriptLine) => ["-e", scriptLine]);

const windowsRecycleBinScript = [
  "Add-Type -AssemblyName Microsoft.VisualBasic;",
  "[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory(",
  "$args[0],",
  "[Microsoft.VisualBasic.FileIO.UIOption]::OnlyErrorDialogs,",
  "[Microsoft.VisualBasic.FileIO.RecycleOption]::SendToRecycleBin",
  ")",
].join(" ");

function detectDesktopPlatform(): ProjectTrashPlatform {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("mac")) {
    return "macos";
  }
  if (platform.includes("win")) {
    return "windows";
  }
  if (platform.includes("linux")) {
    return "linux";
  }
  return "unsupported";
}

const shellCommandRunner: ProjectTrashCommandRunner = {
  async execute(commandName, args) {
    const output = await Command.create(commandName, args).execute();
    return {
      code: output.code,
      signal: output.signal,
      stdout: output.stdout,
      stderr: output.stderr,
    };
  },
};

function trashCommandForPlatform(
  platform: ProjectTrashPlatform,
  projectPath: string,
):
  | {
      commandName: string;
      args: string[];
      mechanism: "finder-trash" | "windows-recycle-bin";
    }
  | null {
  if (platform === "macos") {
    return {
      commandName: "spielgantt-trash-macos",
      args: [...macosFinderTrashScriptArgs, projectPath],
      mechanism: "finder-trash",
    };
  }

  if (platform === "windows") {
    return {
      commandName: "spielgantt-trash-windows",
      args: [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        windowsRecycleBinScript,
        projectPath,
      ],
      mechanism: "windows-recycle-bin",
    };
  }

  return null;
}

export async function moveProjectFolderToTrash(
  projectPath: string,
  deps: ProjectTrashDeps = {},
): Promise<ProjectTrashResult> {
  const trimmedPath = projectPath.trim();
  if (!trimmedPath) {
    return {
      status: "failed",
      projectPath,
      message: "Project deletion needs a project folder path.",
    };
  }

  const platform = (deps.detectPlatform ?? detectDesktopPlatform)();
  const command = trashCommandForPlatform(platform, trimmedPath);
  if (!command) {
    return {
      status: "unsupported",
      projectPath: trimmedPath,
      message:
        "Project deletion is unavailable on this platform because no safe trash/recycle-bin capability is configured.",
    };
  }

  try {
    const output = await (deps.commandRunner ?? shellCommandRunner).execute(
      command.commandName,
      command.args,
    );
    if (output.code !== 0) {
      return {
        status: "failed",
        projectPath: trimmedPath,
        message: output.stderr.trim() || `Trash command exited with code ${output.code}.`,
      };
    }

    return {
      status: "moved-to-trash",
      projectPath: trimmedPath,
      mechanism: command.mechanism,
    };
  } catch (error) {
    return {
      status: "failed",
      projectPath: trimmedPath,
      message: error instanceof Error ? error.message : String(error),
    };
  }
}
