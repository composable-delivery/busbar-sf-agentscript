/**
 * Agent Simulator
 *
 * Runs AgentScript agents locally using mock data, displaying the
 * execution trace in a webview panel. Uses the Rust runtime's WASM bindings
 * via the MCP server's simulation tools or direct LSP custom requests.
 */

import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import type { LanguageClient } from "vscode-languageclient/node";

/** Execution trace from the runtime */
interface ExecutionTrace {
  steps: TraceStep[];
  final_context: Record<string, unknown>;
  outcome: string;
  topic_transitions: string[];
}

interface TraceStep {
  phase: string;
  statement_type: string;
  detail: string;
  variable_changes: VariableChange[];
  action_invocations: ActionInvocation[];
}

interface VariableChange {
  name: string;
  old_value: unknown;
  new_value: unknown;
}

interface ActionInvocation {
  action_name: string;
  inputs: Record<string, unknown>;
  outputs: Record<string, unknown>;
}

export class AgentSimulator {
  private panel: vscode.WebviewPanel | undefined;
  private client: LanguageClient | undefined;
  private disposables: vscode.Disposable[] = [];
  private outputChannel: vscode.OutputChannel;

  constructor() {
    this.outputChannel = vscode.window.createOutputChannel(
      "AgentScript Simulator",
    );
  }

  setClient(client: LanguageClient): void {
    this.client = client;
  }

  async simulate(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "agentscript") {
      vscode.window.showWarningMessage(
        "Open an AgentScript (.agent) file to simulate.",
      );
      return;
    }

    const agentFile = editor.document.uri.fsPath;
    const mockData = await this.loadMockData(agentFile);

    // Request simulation from LSP
    const trace = await this.runSimulation(
      editor.document,
      mockData,
    );

    if (!trace) {
      vscode.window.showErrorMessage(
        "Simulation failed. Check the AgentScript file for errors.",
      );
      return;
    }

    this.showTrace(trace, path.basename(agentFile));
  }

  private async loadMockData(
    agentPath: string,
  ): Promise<Record<string, unknown>> {
    // Convention: look for MyAgent.agent-mocks.json alongside the agent file
    const dir = path.dirname(agentPath);
    const name = path.basename(agentPath, ".agent");

    const candidates = [
      path.join(dir, `${name}.agent-mocks.json`),
      path.join(dir, `${name}.mocks.json`),
      path.join(dir, `${name}-mocks.json`),
    ];

    for (const candidate of candidates) {
      if (fs.existsSync(candidate)) {
        try {
          const raw = fs.readFileSync(candidate, "utf-8");
          return JSON.parse(raw) as Record<string, unknown>;
        } catch (e) {
          vscode.window.showWarningMessage(
            `Failed to parse mock file: ${candidate}`,
          );
        }
      }
    }

    return {};
  }

  private async runSimulation(
    document: vscode.TextDocument,
    mockData: Record<string, unknown>,
  ): Promise<ExecutionTrace | null> {
    if (!this.client) return null;

    try {
      const result = await this.client.sendRequest(
        "agentscript/simulate",
        {
          uri: document.uri.toString(),
          mock_data: mockData,
        },
      );
      return result as ExecutionTrace;
    } catch {
      return null;
    }
  }

  private showTrace(trace: ExecutionTrace, filename: string): void {
    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.Beside);
    } else {
      this.panel = vscode.window.createWebviewPanel(
        "agentscriptSimulator",
        `Simulate: ${filename}`,
        vscode.ViewColumn.Beside,
        { enableScripts: true, retainContextWhenHidden: true },
      );
      this.panel.onDidDispose(() => {
        this.panel = undefined;
      });
    }

    this.panel.webview.html = this.getTraceHtml(trace, filename);

    // Also log to output channel
    this.outputChannel.clear();
    this.outputChannel.appendLine(`=== Simulation: ${filename} ===`);
    this.outputChannel.appendLine(`Outcome: ${trace.outcome}`);
    this.outputChannel.appendLine(
      `Topics visited: ${trace.topic_transitions.join(" → ")}`,
    );
    this.outputChannel.appendLine("");

    for (const step of trace.steps) {
      this.outputChannel.appendLine(
        `[${step.phase}] ${step.statement_type}: ${step.detail}`,
      );
      for (const vc of step.variable_changes) {
        this.outputChannel.appendLine(
          `  Δ ${vc.name}: ${JSON.stringify(vc.old_value)} → ${JSON.stringify(vc.new_value)}`,
        );
      }
      for (const ai of step.action_invocations) {
        this.outputChannel.appendLine(
          `  ⚡ ${ai.action_name}(${JSON.stringify(ai.inputs)}) → ${JSON.stringify(ai.outputs)}`,
        );
      }
    }
  }

  private escapeHtml(text: string): string {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  private getTraceHtml(trace: ExecutionTrace, filename: string): string {
    const stepsHtml = trace.steps
      .map((step, i) => {
        const varChanges = step.variable_changes
          .map(
            (vc) =>
              `<div class="var-change">
              <span class="var-name">${this.escapeHtml(vc.name)}</span>:
              <span class="old-val">${this.escapeHtml(JSON.stringify(vc.old_value))}</span>
              → <span class="new-val">${this.escapeHtml(JSON.stringify(vc.new_value))}</span>
            </div>`,
          )
          .join("");

        const actions = step.action_invocations
          .map(
            (ai) =>
              `<div class="action-invoke">
              ⚡ <span class="action-name">${this.escapeHtml(ai.action_name)}</span>
              <span class="action-io">${this.escapeHtml(JSON.stringify(ai.inputs))} → ${this.escapeHtml(JSON.stringify(ai.outputs))}</span>
            </div>`,
          )
          .join("");

        return `<div class="step">
          <div class="step-header">
            <span class="step-num">#${i + 1}</span>
            <span class="phase phase-${this.escapeHtml(step.phase)}">${this.escapeHtml(step.phase)}</span>
            <span class="stmt-type">${this.escapeHtml(step.statement_type)}</span>
          </div>
          <div class="step-detail">${this.escapeHtml(step.detail)}</div>
          ${varChanges}
          ${actions}
        </div>`;
      })
      .join("");

    const topicPath = trace.topic_transitions
      .map((t) => `<span class="topic-badge">${this.escapeHtml(t)}</span>`)
      .join(" → ");

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: var(--vscode-editor-background);
    color: var(--vscode-editor-foreground);
    font-family: var(--vscode-font-family);
    font-size: 13px;
    padding: 16px;
    overflow-y: auto;
  }
  h1 { font-size: 16px; margin-bottom: 8px; }
  .summary {
    padding: 12px;
    background: var(--vscode-textBlockQuote-background, #2a2a2a);
    border-left: 3px solid var(--vscode-textLink-foreground, #3794ff);
    border-radius: 4px;
    margin-bottom: 16px;
  }
  .summary-row { margin: 4px 0; }
  .label { opacity: 0.7; }
  .topic-badge {
    display: inline-block;
    background: #1a5276;
    color: #89CFF0;
    border-radius: 3px;
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
  }
  .outcome { font-weight: 600; }
  .outcome-success { color: #2ecc71; }
  .outcome-error { color: #e74c3c; }
  .steps { display: flex; flex-direction: column; gap: 8px; }
  .step {
    padding: 10px 12px;
    background: var(--vscode-textBlockQuote-background, #2a2a2a);
    border-radius: 4px;
    border-left: 3px solid #555;
  }
  .step-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
  .step-num { font-weight: 700; opacity: 0.5; font-size: 11px; min-width: 28px; }
  .phase {
    font-size: 10px; font-weight: 600; text-transform: uppercase;
    padding: 1px 6px; border-radius: 3px;
  }
  .phase-before_reasoning { background: #2d4a22; color: #7dce6b; }
  .phase-reasoning { background: #4a3f22; color: #d4ac0d; }
  .phase-after_reasoning { background: #22394a; color: #5dade2; }
  .stmt-type { font-weight: 600; }
  .step-detail { opacity: 0.85; margin-left: 36px; }
  .var-change {
    margin: 4px 0 0 36px; font-size: 12px;
    padding: 2px 6px; background: rgba(255,255,255,0.04); border-radius: 3px;
  }
  .var-name { color: #9b59b6; font-weight: 600; }
  .old-val { color: #e74c3c; text-decoration: line-through; opacity: 0.7; }
  .new-val { color: #2ecc71; }
  .action-invoke {
    margin: 4px 0 0 36px; font-size: 12px;
    padding: 2px 6px; background: rgba(255,255,255,0.04); border-radius: 3px;
  }
  .action-name { color: #1abc9c; font-weight: 600; }
  .action-io { opacity: 0.6; font-size: 11px; }
</style>
</head>
<body>
  <h1>Simulation: ${this.escapeHtml(filename)}</h1>
  <div class="summary">
    <div class="summary-row">
      <span class="label">Outcome:</span>
      <span class="outcome ${trace.outcome === "success" ? "outcome-success" : "outcome-error"}">${this.escapeHtml(trace.outcome)}</span>
    </div>
    <div class="summary-row">
      <span class="label">Topic Path:</span> ${topicPath || "<em>none</em>"}
    </div>
    <div class="summary-row">
      <span class="label">Steps:</span> ${trace.steps.length}
    </div>
  </div>
  <div class="steps">${stepsHtml}</div>
</body>
</html>`;
  }

  dispose(): void {
    this.panel?.dispose();
    this.outputChannel.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}
