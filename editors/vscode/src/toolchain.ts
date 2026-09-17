import * as vscode from "vscode";
import * as fs from "fs";
import * as https from "https";
import * as http from "http";
import * as path from "path";
import * as os from "os";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

const REPO = "ThoriumDC/buraaq";
const STATE_VERSION = "toolchain.version";
const STATE_PROMPTED = "toolchain.installPrompted";

export type ResolvedCli = {
  command: string;
  /** Directory containing buraaq.exe + sysroot (managed install). */
  binDir?: string;
};

function toolchainRoot(context: vscode.ExtensionContext): string {
  return path.join(context.globalStorageUri.fsPath, "toolchain");
}

function managedExe(context: vscode.ExtensionContext): string {
  const name = process.platform === "win32" ? "buraaq.exe" : "buraaq";
  return path.join(toolchainRoot(context), name);
}

function fileExists(p: string): boolean {
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
}

async function whichOnPath(cmd: string): Promise<string | undefined> {
  try {
    if (process.platform === "win32") {
      const { stdout } = await execFileAsync("where.exe", [cmd]);
      const first = stdout.split(/\r?\n/).map((s) => s.trim()).find(Boolean);
      return first && fileExists(first) ? first : undefined;
    }
    const { stdout } = await execFileAsync("which", [cmd]);
    const first = stdout.trim().split(/\n/)[0];
    return first && fileExists(first) ? first : undefined;
  } catch {
    return undefined;
  }
}

/** Prefer explicit setting, then PATH, then extension-managed toolchain. */
export async function resolveCli(
  context: vscode.ExtensionContext,
): Promise<ResolvedCli | undefined> {
  const configured = vscode.workspace.getConfiguration("buraaq").get<string>("lsp.path") ?? "buraaq";

  if (configured !== "buraaq" && path.isAbsolute(configured) && fileExists(configured)) {
    return { command: configured, binDir: path.dirname(configured) };
  }

  if (configured !== "buraaq" && fileExists(configured)) {
    return { command: path.resolve(configured), binDir: path.dirname(path.resolve(configured)) };
  }

  const onPath = await whichOnPath(process.platform === "win32" ? "buraaq.exe" : "buraaq");
  if (onPath) {
    return { command: onPath, binDir: path.dirname(onPath) };
  }

  // `where buraaq` without .exe
  if (process.platform === "win32") {
    const bare = await whichOnPath("buraaq");
    if (bare) {
      return { command: bare, binDir: path.dirname(bare) };
    }
  }

  const managed = managedExe(context);
  if (fileExists(managed)) {
    return { command: managed, binDir: toolchainRoot(context) };
  }

  return undefined;
}

/** Put managed (or resolved) bin dir first on PATH for this window's terminals. */
export function applyTerminalPath(
  context: vscode.ExtensionContext,
  binDir: string | undefined,
): void {
  const col = context.environmentVariableCollection;
  col.clear();
  if (!binDir) {
    return;
  }
  col.prepend("PATH", binDir + path.delimiter);
  col.description = "Buraaq toolchain (extension-managed)";
}

function downloadToFile(url: string, dest: string, redirects = 0): Promise<void> {
  return new Promise((resolve, reject) => {
    if (redirects > 8) {
      reject(new Error("Too many redirects downloading toolchain"));
      return;
    }
    const mod = url.startsWith("https:") ? https : http;
    const req = mod.get(
      url,
      {
        headers: {
          "User-Agent": "ThoriumDC.buraaq-vscode",
          Accept: "application/octet-stream",
        },
      },
      (res) => {
        if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          res.resume();
          downloadToFile(res.headers.location, dest, redirects + 1).then(resolve, reject);
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`Download failed HTTP ${res.statusCode}`));
          return;
        }
        const out = fs.createWriteStream(dest);
        res.pipe(out);
        out.on("finish", () => out.close(() => resolve()));
        out.on("error", reject);
      },
    );
    req.on("error", reject);
  });
}

function httpsJson<T>(url: string): Promise<T> {
  return new Promise((resolve, reject) => {
    https
      .get(
        url,
        {
          headers: {
            "User-Agent": "ThoriumDC.buraaq-vscode",
            Accept: "application/vnd.github+json",
          },
        },
        (res) => {
          if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
            res.resume();
            httpsJson<T>(res.headers.location).then(resolve, reject);
            return;
          }
          const chunks: Buffer[] = [];
          res.on("data", (c) => chunks.push(c));
          res.on("end", () => {
            if (res.statusCode !== 200) {
              reject(new Error(`GitHub API HTTP ${res.statusCode}`));
              return;
            }
            try {
              resolve(JSON.parse(Buffer.concat(chunks).toString("utf8")) as T);
            } catch (e) {
              reject(e);
            }
          });
        },
      )
      .on("error", reject);
  });
}

type GhRelease = {
  tag_name: string;
  assets: { name: string; browser_download_url: string }[];
};

function assetForPlatform(release: GhRelease): { name: string; url: string } | undefined {
  if (process.platform === "win32" && process.arch === "x64") {
    const a = release.assets.find((x) => /buraaq-.*-windows-x64\.zip$/i.test(x.name));
    return a ? { name: a.name, url: a.browser_download_url } : undefined;
  }
  return undefined;
}

async function extractZip(zipPath: string, destDir: string): Promise<void> {
  fs.mkdirSync(destDir, { recursive: true });
  if (process.platform === "win32") {
    const ps = `
$ErrorActionPreference = 'Stop'
Expand-Archive -LiteralPath '${zipPath.replace(/'/g, "''")}' -DestinationPath '${destDir.replace(/'/g, "''")}' -Force
`;
    await execFileAsync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", ps]);
    return;
  }
  await execFileAsync("unzip", ["-o", zipPath, "-d", destDir]);
}

/** Download Release zip into extension globalStorage and return CLI path. */
export async function installManagedToolchain(
  context: vscode.ExtensionContext,
): Promise<ResolvedCli> {
  if (process.platform !== "win32" || process.arch !== "x64") {
    throw new Error(
      "Extension toolchain install currently ships a Windows x64 Release zip only. Use install.sh / a Release for your OS, or set buraaq.lsp.path.",
    );
  }

  const release = await httpsJson<GhRelease>(
    `https://api.github.com/repos/${REPO}/releases/latest`,
  );
  const asset = assetForPlatform(release);
  if (!asset) {
    throw new Error(
      `No Windows x64 zip on ${release.tag_name}. See https://github.com/${REPO}/releases`,
    );
  }

  const root = toolchainRoot(context);
  fs.mkdirSync(context.globalStorageUri.fsPath, { recursive: true });
  fs.mkdirSync(root, { recursive: true });
  const tmpZip = path.join(os.tmpdir(), asset.name);

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: `Buraaq: downloading ${asset.name}`,
      cancellable: false,
    },
    async () => {
      await downloadToFile(asset.url, tmpZip);
      // Clear previous install (keep folder)
      for (const name of fs.readdirSync(root)) {
        fs.rmSync(path.join(root, name), { recursive: true, force: true });
      }
      await extractZip(tmpZip, root);
      try {
        fs.unlinkSync(tmpZip);
      } catch {
        /* ignore */
      }
    },
  );

  const exe = managedExe(context);
  if (!fileExists(exe)) {
    throw new Error(`Download finished but ${exe} is missing (unexpected zip layout).`);
  }
  if (!fs.existsSync(path.join(root, "sysroot"))) {
    throw new Error("Download finished but sysroot/ is missing next to buraaq.exe.");
  }

  await context.globalState.update(STATE_VERSION, release.tag_name);
  applyTerminalPath(context, root);
  return { command: exe, binDir: root };
}

export async function clangOnPath(): Promise<boolean> {
  return !!(await whichOnPath(process.platform === "win32" ? "clang.exe" : "clang"));
}

/** Best-effort LLVM install via winget (Windows). */
export async function installLlvmWinget(): Promise<void> {
  if (process.platform !== "win32") {
    throw new Error("Automatic LLVM install via winget is Windows-only. Install clang another way.");
  }
  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "Buraaq: installing LLVM.LLVM (winget)",
      cancellable: false,
    },
    async () => {
      await execFileAsync(
        "winget",
        [
          "install",
          "--id",
          "LLVM.LLVM",
          "-e",
          "--accept-package-agreements",
          "--accept-source-agreements",
        ],
        { windowsHide: true },
      );
    },
  );
}

export async function ensureToolchain(
  context: vscode.ExtensionContext,
): Promise<ResolvedCli | undefined> {
  let resolved = await resolveCli(context);
  if (resolved) {
    applyTerminalPath(context, resolved.binDir);
    return resolved;
  }

  const auto = vscode.workspace.getConfiguration("buraaq").get<boolean>("toolchain.autoInstall") ?? true;
  if (!auto) {
    return undefined;
  }

  const choice = await vscode.window.showInformationMessage(
    "Buraaq compiler not found. Download the official toolchain (buraaq + sysroot) into this extension?",
    "Install toolchain",
    "Not now",
  );
  await context.globalState.update(STATE_PROMPTED, true);
  if (choice !== "Install toolchain") {
    return undefined;
  }

  resolved = await installManagedToolchain(context);
  return resolved;
}

export async function ensureClangHint(): Promise<void> {
  if (await clangOnPath()) {
    return;
  }
  if (process.platform !== "win32") {
    void vscode.window.showWarningMessage(
      "clang not found on PATH. Buraaq needs LLVM clang to link programs.",
    );
    return;
  }
  const choice = await vscode.window.showWarningMessage(
    "clang not found. Buraaq needs LLVM to build. Install LLVM via winget?",
    "Install LLVM",
    "Later",
  );
  if (choice === "Install LLVM") {
    try {
      await installLlvmWinget();
      void vscode.window.showInformationMessage(
        "LLVM installed. Restart the terminal (and VS Code if clang is still missing) so PATH updates.",
      );
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      void vscode.window.showErrorMessage(
        `LLVM install failed (${msg}). Try: winget install LLVM.LLVM`,
      );
    }
  }
}
