import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { access, mkdir, mkdtemp, readFile, rm, stat, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

const repoRoot = path.resolve(import.meta.dirname, "..");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
const defaultAppPath = path.join(
  repoRoot,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "macos",
  "SpielGantt.app",
);
const defaultDmgDir = path.join(repoRoot, "src-tauri", "target", "release", "bundle", "dmg");
const volumeName = "SpielGantt";

function hostArchForDmg() {
  if (process.env.TAURI_ENV_ARCH) {
    return process.env.TAURI_ENV_ARCH;
  }
  return process.arch === "arm64" ? "aarch64" : process.arch;
}

async function defaultDmgPath() {
  const config = JSON.parse(await readFile(tauriConfigPath, "utf8"));
  return path.join(defaultDmgDir, `SpielGantt_${config.version}_${hostArchForDmg()}.dmg`);
}

async function parseArgs(argv) {
  const args = {
    appPath: defaultAppPath,
    outputPath: await defaultDmgPath(),
    dryRun: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (flag === "--app") {
      if (!value) {
        throw new Error("missing value for --app");
      }
      args.appPath = path.resolve(value);
      index += 1;
    } else if (flag === "--output") {
      if (!value) {
        throw new Error("missing value for --output");
      }
      args.outputPath = path.resolve(value);
      index += 1;
    } else if (flag === "--dry-run") {
      args.dryRun = true;
    } else {
      throw new Error(`unknown argument: ${flag}`);
    }
  }

  return args;
}

async function requireDirectory(directory, description) {
  try {
    await access(directory, constants.F_OK);
  } catch {
    throw new Error(`${description} does not exist: ${directory}`);
  }
  const directoryStat = await stat(directory);
  if (!directoryStat.isDirectory()) {
    throw new Error(`${description} is not a directory: ${directory}`);
  }
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", reject);
    child.on("close", (status) => {
      if (status === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(
          new Error(
            `${command} ${args.join(" ")} failed with status ${status}: ${stderr || stdout}`,
          ),
        );
      }
    });
  });
}

async function packageDmg({ appPath, outputPath, dryRun }) {
  const resolvedAppPath = path.resolve(appPath);
  const resolvedOutputPath = path.resolve(outputPath);
  const stagedAppName = path.basename(resolvedAppPath);
  const plan = {
    appPath: resolvedAppPath,
    outputPath: resolvedOutputPath,
    volumeName,
    contents: [stagedAppName, "Applications"],
    applicationsShortcutTarget: "/Applications",
  };

  if (dryRun) {
    console.log(JSON.stringify(plan, null, 2));
    return;
  }

  if (process.platform !== "darwin") {
    throw new Error("macOS DMG packaging requires macOS");
  }

  await requireDirectory(resolvedAppPath, "app bundle");
  await mkdir(path.dirname(resolvedOutputPath), { recursive: true });

  const tempRoot = await mkdtemp(path.join(tmpdir(), "spielgantt-dmg-"));
  const stagingRoot = path.join(tempRoot, "root");
  try {
    await mkdir(stagingRoot, { recursive: true });
    await runCommand("ditto", [resolvedAppPath, path.join(stagingRoot, stagedAppName)]);
    await symlink("/Applications", path.join(stagingRoot, "Applications"));
    await rm(resolvedOutputPath, { force: true });
    await runCommand("hdiutil", [
      "create",
      "-volname",
      volumeName,
      "-srcfolder",
      stagingRoot,
      "-format",
      "UDZO",
      "-ov",
      resolvedOutputPath,
    ]);
    console.log(`Created macOS DMG: ${resolvedOutputPath}`);
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

try {
  await packageDmg(await parseArgs(process.argv.slice(2)));
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
