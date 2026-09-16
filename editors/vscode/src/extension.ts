import * as path from "path";
import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  const config = workspace.getConfiguration("buraaq.lsp");
  const command = config.get<string>("path") ?? "buraaq";
  const trace = config.get<string>("trace") ?? "off";

  const serverOptions: ServerOptions = {
    run: { command, args: ["lsp-server"], transport: TransportKind.stdio },
    debug: {
      command,
      args: ["lsp-server"],
      transport: TransportKind.stdio,
      options: { env: { RUST_LOG: "buraaq_lsp=debug" } },
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "buraaq" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.bq"),
    },
    traceOutputChannel: undefined,
  };

  client = new LanguageClient("buraaq", "Buraaq Language Server", serverOptions, clientOptions);
  if (trace !== "off") {
    client.setTrace(trace === "verbose" ? 2 : 1);
  }
  context.subscriptions.push(client.start());
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
