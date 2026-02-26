/**
 * Dependency Tree View Provider
 *
 * Shows external Salesforce dependencies (Flows, Apex classes, Prompt Templates,
 * Connections, etc.) extracted from the active AgentScript file, grouped by type.
 *
 * Data comes from the LSP via a custom request that calls the graph crate's
 * extract_dependencies().
 */

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

/** Mirrors DependencyReport from the Rust graph crate */
interface DependencyReport {
  flows: string[];
  apex_classes: string[];
  prompt_templates: string[];
  connections: string[];
  sobjects: string[];
  knowledge_bases: string[];
  external_services: string[];
  all_dependencies: DependencyItem[];
}

interface DependencyItem {
  dep_type: { type: string; name: string };
  used_in: string;
  action_name: string;
  span: [number, number];
}

type TreeItem = CategoryItem | LeafItem;

interface CategoryItem {
  kind: "category";
  label: string;
  icon: string;
  children: LeafItem[];
}

interface LeafItem {
  kind: "leaf";
  name: string;
  category: string;
  usedIn: string;
  actionName: string;
  span: [number, number] | null;
}

export class DependencyTreeProvider
  implements vscode.TreeDataProvider<TreeItem>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    TreeItem | undefined
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  private client: LanguageClient | undefined;
  private cachedReport: DependencyReport | null = null;
  private disposables: vscode.Disposable[] = [];

  constructor() {
    // Refresh on active editor change
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.refresh()),
    );
    // Refresh on save
    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument((doc) => {
        if (doc.languageId === "agentscript") this.refresh();
      }),
    );
  }

  setClient(client: LanguageClient): void {
    this.client = client;
    this.refresh();
  }

  refresh(): void {
    this.cachedReport = null;
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: TreeItem): vscode.TreeItem {
    if (element.kind === "category") {
      const item = new vscode.TreeItem(
        `${element.label} (${element.children.length})`,
        vscode.TreeItemCollapsibleState.Expanded,
      );
      item.iconPath = new vscode.ThemeIcon(element.icon);
      item.contextValue = "category";
      return item;
    }

    // Leaf
    const item = new vscode.TreeItem(
      element.name,
      vscode.TreeItemCollapsibleState.None,
    );
    item.description = `← ${element.usedIn}/${element.actionName}`;
    item.tooltip = `${element.category}: ${element.name}\nUsed in: ${element.usedIn}\nAction: ${element.actionName}`;
    item.iconPath = new vscode.ThemeIcon(this.iconForCategory(element.category));
    if (element.span) {
      item.command = {
        command: "agentscript.navigateToSpan",
        title: "Go to Definition",
        arguments: [element.span[0], element.span[1]],
      };
    }
    return item;
  }

  async getChildren(element?: TreeItem): Promise<TreeItem[]> {
    if (element) {
      return element.kind === "category" ? element.children : [];
    }

    // Root level: build categories
    const report = await this.getDependencies();
    if (!report) {
      return [];
    }

    const categories: CategoryItem[] = [];
    const groups: Record<string, { label: string; icon: string; items: LeafItem[] }> = {
      flow: { label: "Flows", icon: "zap", items: [] },
      apex_class: { label: "Apex Classes", icon: "code", items: [] },
      prompt_template: { label: "Prompt Templates", icon: "comment-discussion", items: [] },
      connection: { label: "Connections", icon: "plug", items: [] },
      sobject: { label: "Objects", icon: "database", items: [] },
      knowledge: { label: "Knowledge Bases", icon: "book", items: [] },
      external_service: { label: "External Services", icon: "globe", items: [] },
    };

    for (const dep of report.all_dependencies) {
      const category = dep.dep_type.type || "custom";
      const group = groups[category];
      const leaf: LeafItem = {
        kind: "leaf",
        name: dep.dep_type.name,
        category,
        usedIn: dep.used_in,
        actionName: dep.action_name,
        span: dep.span,
      };
      if (group) {
        // Avoid duplicate names
        if (!group.items.some((i) => i.name === leaf.name && i.usedIn === leaf.usedIn)) {
          group.items.push(leaf);
        }
      }
    }

    for (const [, group] of Object.entries(groups)) {
      if (group.items.length > 0) {
        categories.push({
          kind: "category",
          label: group.label,
          icon: group.icon,
          children: group.items,
        });
      }
    }

    return categories;
  }

  private async getDependencies(): Promise<DependencyReport | null> {
    if (this.cachedReport) return this.cachedReport;
    if (!this.client) return null;

    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "agentscript") return null;

    try {
      const result = await this.client.sendRequest(
        "agentscript/getDependencies",
        { uri: editor.document.uri.toString() },
      );
      this.cachedReport = result as DependencyReport;
      return this.cachedReport;
    } catch {
      return null;
    }
  }

  private iconForCategory(category: string): string {
    const icons: Record<string, string> = {
      flow: "zap",
      apex_class: "code",
      prompt_template: "comment-discussion",
      connection: "plug",
      sobject: "database",
      knowledge: "book",
      external_service: "globe",
    };
    return icons[category] || "symbol-misc";
  }

  dispose(): void {
    this._onDidChangeTreeData.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}
