"use strict";
/**
 * AgentScript VS Code Extension
 *
 * Activates the AgentScript Language Server (Rust binary) for .agent files.
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const fs = __importStar(require("fs"));
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    // Path to the Rust LSP binary
    // Check user setting first, then look for a bundled binary, then fall back to
    // a binary built in the workspace cargo target directory.
    const config = vscode.workspace.getConfiguration("agentscript");
    const configuredPath = config.get("lsp.serverPath");
    let serverCommand;
    if (configuredPath) {
        serverCommand = configuredPath;
    }
    else {
        // Look for the binary next to the extension (bundled release)
        const bundledPath = vscode.Uri.joinPath(context.extensionUri, "bin", "agentscript-lsp").fsPath;
        if (fs.existsSync(bundledPath)) {
            serverCommand = bundledPath;
        }
        else {
            // Dev fallback: extension lives at <repo>/packages/vscode-agentscript,
            // so the cargo binary is at <repo>/target/debug/agentscript-lsp
            const repoRoot = vscode.Uri.joinPath(context.extensionUri, "..", "..");
            const devPath = vscode.Uri.joinPath(repoRoot, "target", "debug", "agentscript-lsp").fsPath;
            if (fs.existsSync(devPath)) {
                serverCommand = devPath;
            }
            else {
                vscode.window.showErrorMessage(`AgentScript LSP binary not found at ${devPath}. Run 'cargo build -p agentscript-lsp' or set agentscript.lsp.serverPath in settings.`);
                return;
            }
        }
    }
    // Server options - run the language server as an external process
    const serverOptions = {
        run: {
            command: serverCommand,
            transport: node_1.TransportKind.stdio,
        },
        debug: {
            command: serverCommand,
            transport: node_1.TransportKind.stdio,
        },
    };
    // Client options - configure which documents the language server handles
    const clientOptions = {
        documentSelector: [
            { scheme: "file", language: "agentscript" },
            { scheme: "untitled", language: "agentscript" },
        ],
        synchronize: {
            // Notify the server about file changes to .agent files
            fileEvents: vscode.workspace.createFileSystemWatcher("**/*.agent"),
        },
        outputChannelName: "AgentScript Language Server",
    };
    // Create the language client
    client = new node_1.LanguageClient("agentscriptLanguageServer", "AgentScript Language Server", serverOptions, clientOptions);
    // Register restart command
    const restartCommand = vscode.commands.registerCommand("agentscript.restartServer", async () => {
        if (client) {
            await client.restart();
            vscode.window.showInformationMessage("AgentScript Language Server restarted");
        }
    });
    context.subscriptions.push(restartCommand);
    // Start the client (this also launches the server)
    client.start();
    console.log("AgentScript extension activated with Rust LSP");
}
async function deactivate() {
    if (client) {
        await client.stop();
    }
}
//# sourceMappingURL=extension.js.map