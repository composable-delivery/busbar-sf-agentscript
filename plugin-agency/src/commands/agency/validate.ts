import { SfCommand, Flags, Ux } from "@salesforce/sf-plugins-core";
import { Messages } from "@salesforce/core";
import * as fs from "fs";
import * as path from "path";
import ansis from "ansis";
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../wasm-loader.js';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graph from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages(
  "@muselab/sf-plugin-busbar-agency",
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
  file: string;
  valid: boolean;
  issues: ValidationIssue[];
};

export default class AgentscriptValidate extends SfCommand<ValidationResult | ValidationResult[]> {
  public static readonly summary = messages.getMessage("summary");
  public static readonly description = messages.getMessage("description");
  public static readonly examples = messages.getMessages("examples");

  public static readonly flags = {
    file: Flags.file({
      char: "f",
      summary: messages.getMessage("flags.file.summary"),
      description: messages.getMessage("flags.file.description"),
      required: false,
      exists: true,
    }),
    path: Flags.directory({
      summary: 'Directory to scan for agent files (default: current directory).',
      description: 'Recursively searches this directory for .agent files when --file is not specified.',
      default: '.',
    }),
    verbose: Flags.boolean({
      summary: 'Show section headers and full formatting per file.',
      default: false,
    }),
  };

  public async run(): Promise<ValidationResult | ValidationResult[]> {
    const { flags } = await this.parse(AgentscriptValidate);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const files = resolveTargetFiles({
      file: flags.file,
      scanPath: flags.path,
      dataDir: this.config.dataDir,
    });

    // Read all files in parallel
    const fileReads = await Promise.all(
      files.map(async (filePath) => {
        try {
          const source = await fs.promises.readFile(filePath, 'utf-8');
          return { filePath, source, ok: true as const };
        } catch (e) {
          return { filePath, source: '', ok: false as const, error: e instanceof Error ? e.message : String(e) };
        }
      })
    );

    const results: ValidationResult[] = [];
    const fileErrors: Array<{ file: string; error: string }> = [];

    for (const fileRead of fileReads) {
      const file = path.relative(process.cwd(), fileRead.filePath);

      if (!fileRead.ok) {
        fileErrors.push({ file, error: fileRead.error });
        continue;
      }

      if (files.length > 1) {
        this.log(ansis.bold.dim(`\n─── ${file} ───`));
      }

      try {
        const result = this.validateSource(fileRead.source, file, fileRead.filePath, ux, flags.verbose);
        results.push(result);
        if (!result.valid) process.exitCode = 1;
      } catch (e) {
        fileErrors.push({ file, error: e instanceof Error ? e.message : String(e) });
      }
    }

    if (fileErrors.length > 0) {
      this.log('');
      this.log(ansis.red.bold(`${fileErrors.length} file${fileErrors.length === 1 ? '' : 's'} failed to validate:`));
      for (const { file, error } of fileErrors) {
        this.log(`  ${ansis.red('✗')} ${ansis.bold(file)}: ${ansis.dim(error)}`);
      }
      process.exitCode = 1;
    }

    return files.length === 1 ? results[0] : results;
  }

  private validateSource(
    source: string,
    file: string,
    filePath: string,
    ux: Ux,
    verbose: boolean,
  ): ValidationResult {
    const fileName = path.basename(filePath);

    const getLineCol = (offset: number) => {
      const prefix = source.substring(0, offset);
      const line = prefix.split("\n").length;
      const lastNewLine = prefix.lastIndexOf("\n");
      const column = lastNewLine === -1 ? offset + 1 : offset - lastNewLine;
      return { line, column };
    };

    const startTime = performance.now();
    const issues: ValidationIssue[] = [];

    // 1. Semantic Validation
    const semanticResult = parser.validate_agent_semantic(source);

    if (semanticResult.errors) {
      semanticResult.errors.forEach((e: any) => {
        const issue: ValidationIssue = { message: e.message, severity: "Error", hint: e.hint };
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
        const issue: ValidationIssue = { message: w.message, severity: "Warning", hint: w.hint };
        if (w.span) {
          const { line, column } = getLineCol(w.span.start);
          issue.line = line;
          issue.column = column;
        }
        issues.push(issue);
      });
    }

    // 2. Graph Validation (only if no parse errors)
    const parseErrors = issues.filter(
      (i) => i.severity === "Error" && i.message.includes("Parse error"),
    );

    if (parseErrors.length === 0) {
      try {
        const graphResult = graph.validate_graph(source);
        if (graphResult.errors) {
          graphResult.errors.forEach((e: any) => {
            const issue: ValidationIssue = { message: e.message, severity: "Error" };
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
            const issue: ValidationIssue = { message: w.message, severity: "Warning" };
            if (w.span_start !== undefined) {
              const { line, column } = getLineCol(w.span_start);
              issue.line = line;
              issue.column = column;
            }
            issues.push(issue);
          });
        }
      } catch {
        // Graph validation failed — rely on semantic check results
      }
    }

    const elapsed = (performance.now() - startTime).toFixed(2);
    const isValid = !issues.some((i) => i.severity === "Error");

    if (verbose) {
      ux.styledHeader("Validation Result");
      this.log('');
    }

    if (isValid) {
      this.log(
        `  ${ansis.green("✓")} ${ansis.bold(fileName)} ${ansis.greenBright("valid")} ${ansis.dim(`(${elapsed}ms)`)}`,
      );
    } else {
      const errorCount = issues.filter(i => i.severity === 'Error').length;
      const warnCount = issues.filter(i => i.severity === 'Warning').length;
      const summary = [
        errorCount > 0 ? ansis.red(`${errorCount} error${errorCount === 1 ? '' : 's'}`) : '',
        warnCount > 0 ? ansis.yellow(`${warnCount} warning${warnCount === 1 ? '' : 's'}`) : '',
      ].filter(Boolean).join(', ');
      this.log(`  ${ansis.red("✗")} ${ansis.bold(fileName)} ${ansis.redBright("invalid")}  ${summary}  ${ansis.dim(`(${elapsed}ms)`)}`);
    }

    if (issues.length > 0) {
      this.log('');
      const tableData = issues.map((row) => ({
        type: row.severity === "Error" ? ansis.red(row.severity) : ansis.yellow(row.severity),
        location: row.line ? `L${row.line}:C${row.column}` : "-",
        message: row.message,
        hint: row.hint ? ansis.dim(row.hint) : "",
      }));
      ux.table({
        data: tableData,
        columns: [
          { key: 'type', name: 'Type' },
          { key: 'location', name: 'Location' },
          { key: 'message', name: 'Message' },
          { key: 'hint', name: 'Hint' },
        ],
      });
      this.log('');
    }

    return { file, valid: isValid, issues };
  }
}
