/**
 * AgentScript VS Code Extension
 *
 * Full-featured extension providing:
 * - Language Server (Rust binary) for parsing, diagnostics, completions, etc.
 * - Graph visualization webview showing topic flow DAG
 * - Dependency tree view showing external Salesforce dependencies
 * - Agent simulation with mock data and execution trace
 */

import * as fs from "fs";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import { GraphWebviewProvider } from "./graphWebview";
import { DependencyTreeProvider } from "./dependencyTree";
import { AgentSimulator } from "./simulator";

let client: LanguageClient | undefined;
let graphProvider: GraphWebviewProvider | undefined;
let dependencyProvider: DependencyTreeProvider | undefined;
let simulator: AgentSimulator | undefined;

export function activate(context: vscode.ExtensionContext): void {
  // ── LSP Binary Resolution ──────────────────────────────────────────
  const config = vscode.workspace.getConfiguration("agentscript");
  const configuredPath = config.get<string>("lsp.serverPath");

  let serverCommand: string;
  if (configuredPath) {
    serverCommand = configuredPath;
  } else {
    const bundledPath = vscode.Uri.joinPath(
      context.extensionUri,
      "bin",
      "agentscript-lsp",
    ).fsPath;

    if (fs.existsSync(bundledPath)) {
      serverCommand = bundledPath;
    } else {
      const repoRoot = vscode.Uri.joinPath(context.extensionUri, "..", "..");
      const devPath = vscode.Uri.joinPath(
        repoRoot,
        "target",
        "debug",
        "agentscript-lsp",
      ).fsPath;

      if (fs.existsSync(devPath)) {
        serverCommand = devPath;
      } else {
        vscode.window.showErrorMessage(
          `AgentScript LSP binary not found at ${devPath}. Run 'cargo build -p agentscript-lsp' or set agentscript.lsp.serverPath in settings.`,
        );
        return;
      }
    }
  }

  // ── Language Server ────────────────────────────────────────────────
  const serverOptions: ServerOptions = {
    run: { command: serverCommand, transport: TransportKind.stdio },
    debug: { command: serverCommand, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "agentscript" },
      { scheme: "untitled", language: "agentscript" },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.agent"),
    },
    outputChannelName: "AgentScript Language Server",
  };

  client = new LanguageClient(
    "agentscriptLanguageServer",
    "AgentScript Language Server",
    serverOptions,
    clientOptions,
  );

  // ── Graph Visualization ────────────────────────────────────────────
  graphProvider = new GraphWebviewProvider(context.extensionUri);
  graphProvider.setClient(client);

  // ── Dependency Tree View ───────────────────────────────────────────
  dependencyProvider = new DependencyTreeProvider();
  const treeView = vscode.window.createTreeView("agentscriptDependencies", {
    treeDataProvider: dependencyProvider,
    showCollapseAll: true,
  });
  context.subscriptions.push(treeView);

  // ── Agent Simulator ────────────────────────────────────────────────
  simulator = new AgentSimulator();
  simulator.setClient(client);

  // ── Commands ───────────────────────────────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand("agentscript.restartServer", async () => {
      if (client) {
        await client.restart();
        vscode.window.showInformationMessage(
          "AgentScript Language Server restarted",
        );
      }
    }),

    vscode.commands.registerCommand("agentscript.showGraph", () => {
      graphProvider?.show();
    }),

    vscode.commands.registerCommand("agentscript.refreshDependencies", () => {
      dependencyProvider?.refresh();
    }),

    vscode.commands.registerCommand("agentscript.simulate", () => {
      simulator?.simulate();
    }),

    vscode.commands.registerCommand(
      "agentscript.navigateToSpan",
      (spanStart: number, spanEnd: number) => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        const startPos = editor.document.positionAt(spanStart);
        const endPos = editor.document.positionAt(spanEnd);
        editor.revealRange(
          new vscode.Range(startPos, endPos),
          vscode.TextEditorRevealType.InCenter,
        );
        editor.selection = new vscode.Selection(startPos, startPos);
      },
    ),
  );

  // ── Start LSP ─────────────────────────────────────────────────────
  client.start().then(() => {
    // Wire up the client to features that need it after it's ready
    if (client) {
      dependencyProvider?.setClient(client);
    }
  });

  console.log("AgentScript extension activated with Rust LSP");
}

export async function deactivate(): Promise<void> {
  graphProvider?.dispose();
  dependencyProvider?.dispose();
  simulator?.dispose();
  if (client) {
    await client.stop();
  }
}
