import { SfCommand, Flags, Ux } from "@salesforce/sf-plugins-core";
import { Messages } from "@salesforce/core";
import * as fs from "fs";
import * as path from "path";
import ansis from "ansis";
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from "busbar-sf-agentscript";
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graph from "busbar-sf-agentscript";

// After bundling, __dirname is lib/commands/agentscript-parser/ - go up 3 levels to plugin root
const pluginRoot = path.resolve(__dirname, "..", "..", "..");
Messages.importMessagesDirectory(pluginRoot);
const messages = Messages.loadMessages(
  "sf-plugin-busbar-agency",
  "agency.validate",
);

export type ValidationIssue = {
  message: string;
  severity: "Error" | "Warning";
  line?: number;
  column?: number;
  hint?: string;
};

export type ValidationResult = {
  valid: boolean;
  issues: ValidationIssue[];
};

export default class AgentscriptValidate extends SfCommand<ValidationResult> {
  public static readonly summary = messages.getMessage("summary");
  public static readonly description = messages.getMessage("description");
  public static readonly examples = messages.getMessages("examples");

  public static readonly flags = {
    file: Flags.file({
      char: "f",
      summary: messages.getMessage("flags.file.summary"),
      description: messages.getMessage("flags.file.description"),
      required: true,
      exists: true,
    }),
  };

  public async run(): Promise<ValidationResult> {
    const { flags } = await this.parse(AgentscriptValidate);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      // Read the AgentScript file
      const filePath = path.resolve(flags.file as string);
      const source = fs.readFileSync(filePath, "utf-8");
      const fileName = path.basename(filePath);

      // Helper to calculate line/col from offset
      const getLineCol = (offset: number) => {
        const prefix = source.substring(0, offset);
        const line = prefix.split("\n").length;
        const lastNewLine = prefix.lastIndexOf("\n");
        const column = lastNewLine === -1 ? offset + 1 : offset - lastNewLine;
        return { line, column };
      };

      // Validate using WASM with timing
      const startTime = performance.now();
      const issues: ValidationIssue[] = [];

      // 1. Semantic Validation
      const semanticResult = parser.validate_agent_semantic(source);

      if (semanticResult.errors) {
        semanticResult.errors.forEach((e: any) => {
          const issue: ValidationIssue = {
            message: e.message,
            severity: "Error",
            hint: e.hint,
          };
          if (e.span) {
            const { line, column } = getLineCol(e.span.start);
            issue.line = line;
            issue.column = column;
          }
          issues.push(issue);
        });
      }
      if (semanticResult.warnings) {
        semanticResult.warnings.forEach((w: any) => {
          const issue: ValidationIssue = {
            message: w.message,
            severity: "Warning",
            hint: w.hint,
          };
          if (w.span) {
            const { line, column } = getLineCol(w.span.start);
            issue.line = line;
            issue.column = column;
          }
          issues.push(issue);
        });
      }

      // 2. Graph Validation (only if no parse errors)
      // If there are semantic errors that are parse errors, graph validation might fail
      const parseErrors = issues.filter(
        (i) => i.severity === "Error" && i.message.includes("Parse error"),
      );

      if (parseErrors.length === 0) {
        try {
          const graphResult = graph.validate_graph(source);
          if (graphResult.errors) {
            graphResult.errors.forEach((e: any) => {
              const issue: ValidationIssue = {
                message: e.message,
                severity: "Error",
              };
              if (e.span_start !== undefined) {
                const { line, column } = getLineCol(e.span_start);
                issue.line = line;
                issue.column = column;
              }
              issues.push(issue);
            });
          }
          if (graphResult.warnings) {
            graphResult.warnings.forEach((w: any) => {
              const issue: ValidationIssue = {
                message: w.message,
                severity: "Warning",
              };
              if (w.span_start !== undefined) {
                const { line, column } = getLineCol(w.span_start);
                issue.line = line;
                issue.column = column;
              }
              issues.push(issue);
            });
          }
        } catch (e) {
          // Graph validation failed, possibly due to structure issues not caught by semantic check
          // We'll rely on the parser semantic checks to have reported major issues
        }
      }

      const elapsed = (performance.now() - startTime).toFixed(2);
      const isValid = !issues.some((i) => i.severity === "Error");

      ux.styledHeader("Validation Result");
      this.log("");

      if (isValid) {
        this.log(
          `  ${ansis.green("✓")} ${ansis.bold(fileName)} is ${ansis.greenBright("valid")} AgentScript`,
        );
      } else {
        this.log(
          `  ${ansis.red("✗")} ${ansis.bold(fileName)} has ${ansis.redBright("errors")}`,
        );
      }

      this.log(`    ${ansis.dim(`Validated in ${elapsed}ms`)}`);
      this.log("");

      if (issues.length > 0) {
        ux.table(issues, {
          severity: {
            header: "Type",
            get: (row) =>
              row.severity === "Error"
                ? ansis.red(row.severity)
                : ansis.yellow(row.severity),
          },
          location: {
            header: "Location",
            get: (row) => (row.line ? `L${row.line}:C${row.column}` : "-"),
          },
          message: { header: "Message" },
          hint: {
            header: "Hint",
            get: (row) => (row.hint ? ansis.dim(row.hint) : ""),
          },
        });
        this.log("");
      }

      if (!isValid) {
        process.exitCode = 1;
      }

      return { valid: isValid, issues };
    } catch (error) {
      if (error instanceof Error) {
        ux.styledHeader("Validation Result");
        this.log("");
        this.log(`  ${ansis.red("✗")} ${ansis.redBright("Validation failed")}`);
        this.log(`    ${ansis.dim(error.message)}`);
        this.log("");
        this.error(error.message);
      }
      throw error;
    }
  }
}
