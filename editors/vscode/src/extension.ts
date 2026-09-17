import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

async function startClient(context: vscode.ExtensionContext): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }

  const config = vscode.workspace.getConfiguration("buraaq");
  const command = config.get<string>("lsp.path") ?? "buraaq";
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

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("buraaq.restartServer", async () => {
      try {
        await startClient(context);
        void vscode.window.showInformationMessage("Buraaq language server restarted.");
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(`Buraaq LSP restart failed: ${msg}`);
      }
    }),
  );

  try {
    await startClient(context);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    void vscode.window.showWarningMessage(
      `Buraaq LSP did not start (${msg}). Syntax highlighting still works. Install Buraaq and ensure \`buraaq\` is on PATH.`,
    );
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
