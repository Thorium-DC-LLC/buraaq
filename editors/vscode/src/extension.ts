import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import {
  ensureClangHint,
  ensureToolchain,
  installLlvmWinget,
  installManagedToolchain,
  resolveCli,
} from "./toolchain";

let client: LanguageClient | undefined;

async function startClient(
  context: vscode.ExtensionContext,
  command: string,
): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }

  const config = vscode.workspace.getConfiguration("buraaq");
  const trace = config.get<string>("lsp.trace") ?? "off";

  const serverOptions: ServerOptions = {
    run: { command, args: ["lsp-server"], transport: TransportKind.stdio },
    debug: {
      command,
      args: ["lsp-server"],
      transport: TransportKind.stdio,
      options: { env: { ...process.env, RUST_LOG: "buraaq_lsp=debug" } },
    },
  };

  const output = vscode.window.createOutputChannel("Buraaq");
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "buraaq" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{bq,pkg}"),
    },
    outputChannel: output,
  };

  client = new LanguageClient("buraaq", "Buraaq Language Server", serverOptions, clientOptions);
  if (trace !== "off") {
    await client.setTrace(trace === "verbose" ? 2 : 1);
  }

  await client.start();
  context.subscriptions.push(output);
}

async function bootLanguageServer(context: vscode.ExtensionContext): Promise<void> {
  const resolved = await ensureToolchain(context);
  if (!resolved) {
    void vscode.window.showWarningMessage(
      "Buraaq compiler not found. Run “Buraaq: Install Compiler Toolchain”, or set `buraaq.lsp.path`.",
    );
    return;
  }
  await startClient(context, resolved.command);
  void ensureClangHint();
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("buraaq.restartServer", async () => {
      try {
        const resolved = (await resolveCli(context)) ?? (await ensureToolchain(context));
        if (!resolved) {
          void vscode.window.showErrorMessage("Buraaq CLI not found.");
          return;
        }
        await startClient(context, resolved.command);
        void vscode.window.showInformationMessage("Buraaq language server restarted.");
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(`Buraaq LSP restart failed: ${msg}`);
      }
    }),
    vscode.commands.registerCommand("buraaq.installToolchain", async () => {
      try {
        const resolved = await installManagedToolchain(context);
        await startClient(context, resolved.command);
        void vscode.window.showInformationMessage(
          `Buraaq toolchain installed (${resolved.command}). Integrated terminals pick it up via PATH.`,
        );
        void ensureClangHint();
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(`Toolchain install failed: ${msg}`);
      }
    }),
    vscode.commands.registerCommand("buraaq.installLlvm", async () => {
      try {
        await installLlvmWinget();
        void vscode.window.showInformationMessage(
          "LLVM installed. Restart terminals (or VS Code) if `clang` is still not found.",
        );
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(`LLVM install failed: ${msg}`);
      }
    }),
  );

  try {
    await bootLanguageServer(context);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    void vscode.window.showWarningMessage(
      `Buraaq LSP did not start (${msg}). Syntax highlighting still works. Run “Buraaq: Install Compiler Toolchain”.`,
    );
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
