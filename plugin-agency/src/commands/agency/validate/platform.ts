import { SfCommand, Flags, Ux } from "@salesforce/sf-plugins-core";
import { Messages } from "@salesforce/core";
import * as fs from "fs";
import * as path from "path";
import { execSync } from "child_process";
import ansis from "ansis";
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from "busbar-sf-agentscript";
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graph from "busbar-sf-agentscript";

// After bundling, __dirname is lib/commands/agency/validate/ - go up 4 levels to plugin root
const pluginRoot = path.resolve(__dirname, "..", "..", "..", "..");
Messages.importMessagesDirectory(pluginRoot);
const messages = Messages.loadMessages(
  "sf-plugin-busbar-agency",
  "agency.validate.platform",
);

interface PlatformIssue {
  message: string;
  line: number;
  column: number;
  severity: "error" | "warning";
}

interface PlatformValidateResult {
  platform: {
    success: boolean;
    issues: PlatformIssue[];
  };
  local?: {
    valid: boolean;
    issues: any[];
  };
  injectedDefaultUser: boolean;
}

export default class ValidatePlatform extends SfCommand<PlatformValidateResult> {
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
    "target-org": Flags.requiredOrg({
      summary: messages.getMessage("flags.target-org.summary"),
      description: messages.getMessage("flags.target-org.description"),
    }),
    "skip-local": Flags.boolean({
      summary: messages.getMessage("flags.skip-local.summary"),
      description: messages.getMessage("flags.skip-local.description"),
      default: false,
    }),
  };

  public async run(): Promise<PlatformValidateResult> {
    const { flags } = await this.parse(ValidatePlatform);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const filePath = path.resolve(flags.file as string);
    const source = fs.readFileSync(filePath, "utf-8");
    const fileName = path.basename(filePath);
    const org = flags["target-org"];
    const orgAlias = org.getUsername() ?? "";

    // Extract agent_name from config block
    const agentName = extractAgentName(source);
    if (!agentName) {
      this.error(messages.getMessage("error.noAgentName"));
    }

    // Check if default_agent_user is present
    let modifiedSource = source;
    let injectedDefaultUser = false;
    if (!hasDefaultAgentUser(source)) {
      modifiedSource = injectDefaultAgentUser(source, orgAlias);
      injectedDefaultUser = true;
    }

    // Create temp DX project
    const tmpDir = fs.mkdtempSync(
      path.join(require("os").tmpdir(), "agency-validate-"),
    );
    let platformResult: PlatformValidateResult["platform"] = {
      success: false,
      issues: [],
    };

    try {
      if (!this.jsonEnabled()) {
        ux.styledHeader("Platform Validation");
        this.log("");
        this.log(`  ${ansis.dim("File:")} ${ansis.bold(fileName)}`);
        this.log(`  ${ansis.dim("Agent:")} ${ansis.bold(agentName)}`);
        this.log(`  ${ansis.dim("Org:")} ${ansis.bold(orgAlias)}`);
        if (injectedDefaultUser) {
          this.log(
            `  ${ansis.yellow("!")} Injected default_agent_user: ${ansis.cyan(orgAlias)}`,
          );
        }
        this.log("");
      }

      // Build temp project structure
      createTempProject(tmpDir, agentName, modifiedSource);

      // Run platform validation
      const startTime = performance.now();
      platformResult = runPlatformValidation(tmpDir, agentName, orgAlias);
      const elapsed = (performance.now() - startTime).toFixed(0);

      if (!this.jsonEnabled()) {
        this.displayPlatformResults(platformResult, elapsed);
      }
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError("error.platformValidation", [error.message]);
      }
      throw error;
    } finally {
      // Clean up temp directory
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }

    // Run local WASM validation unless skipped
    let localResult: { valid: boolean; issues: any[] } | undefined;
    if (!flags["skip-local"]) {
      try {
        const localStart = performance.now();
        const issues: any[] = [];

        // 1. Semantic Validation
        const semanticResult = parser.validate_agent_semantic(source);

        if (semanticResult.errors) {
          semanticResult.errors.forEach((e: any) => {
            const lineCol = e.span
              ? getLineCol(source, e.span.start)
              : undefined;
            issues.push({
              message: e.message,
              severity: "Error",
              hint: e.hint,
              line: lineCol?.line,
              column: lineCol?.column,
            });
          });
        }
        if (semanticResult.warnings) {
          semanticResult.warnings.forEach((w: any) => {
            const lineCol = w.span
              ? getLineCol(source, w.span.start)
              : undefined;
            issues.push({
              message: w.message,
              severity: "Warning",
              hint: w.hint,
              line: lineCol?.line,
              column: lineCol?.column,
            });
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
                const lineCol =
                  e.span_start !== undefined
                    ? getLineCol(source, e.span_start)
                    : undefined;
                issues.push({
                  message: e.message,
                  severity: "Error",
                  line: lineCol?.line,
                  column: lineCol?.column,
                });
              });
            }
            if (graphResult.warnings) {
              graphResult.warnings.forEach((w: any) => {
                const lineCol =
                  w.span_start !== undefined
                    ? getLineCol(source, w.span_start)
                    : undefined;
                issues.push({
                  message: w.message,
                  severity: "Warning",
                  line: lineCol?.line,
                  column: lineCol?.column,
                });
              });
            }
          } catch (e) {
            // Graph validation failed
          }
        }

        const localElapsed = (performance.now() - localStart).toFixed(2);
        const isValid = !issues.some((i) => i.severity === "Error");
        localResult = { valid: isValid, issues };

        if (!this.jsonEnabled()) {
          this.log("");
          ux.styledHeader("Local WASM Validation");
          this.log("");
          if (isValid) {
            this.log(
              `  ${ansis.green("✓")} Local syntax: ${ansis.greenBright("valid")}`,
            );
          } else {
            this.log(
              `  ${ansis.red("✗")} Local syntax: ${ansis.redBright("invalid")}`,
            );
          }
          this.log(`    ${ansis.dim(`Validated in ${localElapsed}ms`)}`);
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
                get: (row: any) =>
                  row.line ? `L${row.line}:C${row.column}` : "-",
              },
              message: { header: "Message" },
              hint: {
                header: "Hint",
                get: (row: any) => (row.hint ? ansis.dim(row.hint) : ""),
              },
            });
            this.log("");
          }
        }
      } catch (error) {
        localResult = { valid: false, issues: [] };
        const errorMsg = error instanceof Error ? error.message : String(error);
        if (!this.jsonEnabled()) {
          this.log("");
          ux.styledHeader("Local WASM Validation");
          this.log("");
          this.log(`  ${ansis.red("✗")} ${ansis.redBright("Parse error:")}`);
          this.log(`    ${ansis.dim(errorMsg)}`);
          this.log("");
        }
      }
    }

    return {
      platform: platformResult,
      local: localResult,
      injectedDefaultUser,
    };
  }

  private displayPlatformResults(
    result: PlatformValidateResult["platform"],
    elapsed: string,
  ): void {
    if (result.success && result.issues.length === 0) {
      this.log(
        `  ${ansis.green("✓")} Platform validation: ${ansis.greenBright("passed")}`,
      );
      this.log(`    ${ansis.dim(`Validated in ${elapsed}ms`)}`);
    } else {
      const errors = result.issues.filter((i) => i.severity === "error");
      const warnings = result.issues.filter((i) => i.severity === "warning");

      if (errors.length > 0) {
        this.log(
          `  ${ansis.red("✗")} Platform validation: ${ansis.redBright("failed")}`,
        );
      } else {
        this.log(
          `  ${ansis.yellow("!")} Platform validation: ${ansis.yellowBright("warnings")}`,
        );
      }
      this.log(`    ${ansis.dim(`Validated in ${elapsed}ms`)}`);
      this.log("");

      for (const issue of result.issues) {
        const icon =
          issue.severity === "error" ? ansis.red("✗") : ansis.yellow("!");
        // Only append location if it's not already embedded in the message
        const hasEmbeddedLoc = /\[Ln\s*\d+/.test(issue.message);
        const loc =
          issue.line > 0 && !hasEmbeddedLoc
            ? ansis.dim(` [Ln ${issue.line}, Col ${issue.column}]`)
            : "";
        this.log(`  ${icon} ${issue.message}${loc}`);
      }
    }
  }
}

// ============================================
// Helper Functions
// ============================================

function extractAgentName(source: string): string | undefined {
  // Match agent_name: "Name" in the config block
  const match = source.match(/agent_name:\s*"([^"]+)"/);
  return match?.[1];
}

function hasDefaultAgentUser(source: string): boolean {
  return /default_agent_user:/.test(source);
}

function injectDefaultAgentUser(source: string, username: string): string {
  // Find the config block and inject default_agent_user after agent_type line
  // or after agent_name if no agent_type
  const lines = source.split("\n");
  const result: string[] = [];

  let inConfig = false;
  let injected = false;

  for (const line of lines) {
    result.push(line);

    if (line.match(/^config:/)) {
      inConfig = true;
      continue;
    }

    if (inConfig && !injected) {
      // Inject after agent_type or agent_name line
      if (line.match(/^\s+agent_type:/) || line.match(/^\s+agent_name:/)) {
        // Detect indentation from current line
        const indent = line.match(/^(\s+)/)?.[1] || "   ";
        result.push(`${indent}default_agent_user: "${username}"`);
        injected = true;
      }
    }

    // Exit config block when we hit a non-indented line (next top-level block)
    if (inConfig && injected && line.match(/^\S/) && !line.match(/^config:/)) {
      inConfig = false;
    }
  }

  return result.join("\n");
}

function createTempProject(
  tmpDir: string,
  agentName: string,
  source: string,
): void {
  // sfdx-project.json
  const projectJson = {
    packageDirectories: [{ path: "force-app", default: true }],
    sourceApiVersion: "63.0",
  };
  fs.writeFileSync(
    path.join(tmpDir, "sfdx-project.json"),
    JSON.stringify(projectJson, null, 2),
  );

  // Agent bundle directory
  const bundleDir = path.join(
    tmpDir,
    "force-app",
    "main",
    "default",
    "aiAuthoringBundles",
    agentName,
  );
  fs.mkdirSync(bundleDir, { recursive: true });

  // Agent file
  fs.writeFileSync(path.join(bundleDir, `${agentName}.agent`), source);

  // Bundle metadata
  const bundleMeta = `<?xml version="1.0" encoding="UTF-8" ?>
<AiAuthoringBundle xmlns="http://soap.sforce.com/2006/04/metadata">
    <bundleType>AGENT</bundleType>
    <versionTag>v0.1</versionTag>
</AiAuthoringBundle>`;
  fs.writeFileSync(
    path.join(bundleDir, `${agentName}.bundle-meta.xml`),
    bundleMeta,
  );
}

function runPlatformValidation(
  tmpDir: string,
  agentName: string,
  orgAlias: string,
): PlatformValidateResult["platform"] {
  try {
    const cmd = `sf agent validate authoring-bundle --api-name ${agentName} --target-org ${orgAlias} --json`;
    const output = execSync(cmd, {
      cwd: tmpDir,
      encoding: "utf-8",
      timeout: 120_000,
      stdio: ["pipe", "pipe", "pipe"],
    });

    const json = JSON.parse(output);

    // sf cli --json wraps result in { status, result, warnings }
    if (json.status === 0) {
      return { success: true, issues: [] };
    }

    return parsePlatformOutput(json);
  } catch (error: unknown) {
    // execSync throws on non-zero exit code, but the output is still in stderr/stdout
    const execError = error as {
      stdout?: string;
      stderr?: string;
      message?: string;
    };

    // Try to parse JSON from stdout (sf cli writes JSON to stdout even on failure)
    if (execError.stdout) {
      try {
        const json = JSON.parse(execError.stdout);
        return parsePlatformOutput(json);
      } catch {
        // Fall through to stderr parsing
      }
    }

    // Try to parse from stderr
    if (execError.stderr) {
      try {
        const json = JSON.parse(execError.stderr);
        return parsePlatformOutput(json);
      } catch {
        // Fall through to raw error
      }
    }

    // Return raw error message
    return {
      success: false,
      issues: [
        {
          message: execError.message || "Unknown platform validation error",
          line: 0,
          column: 0,
          severity: "error",
        },
      ],
    };
  }
}

function getLineCol(
  source: string,
  offset: number,
): { line: number; column: number } {
  const prefix = source.substring(0, offset);
  const line = prefix.split("\n").length;
  const lastNewLine = prefix.lastIndexOf("\n");
  const column = lastNewLine === -1 ? offset + 1 : offset - lastNewLine;
  return { line, column };
}

function parsePlatformOutput(
  json: Record<string, unknown>,
): PlatformValidateResult["platform"] {
  const issues: PlatformIssue[] = [];

  // Handle sf cli error format: { status: 1, name: "...", message: "...", ... }
  if (json.message && typeof json.message === "string") {
    // Platform returns errors in the message field, often with line/column info
    const msg = json.message as string;

    // Try to parse structured error messages like "Syntax error at [Ln X, Col Y]: message"
    const errorPattern =
      /(?:Syntax error|Error|Missing required element|Unexpected '[^']*').*?\[Ln\s*(\d+),\s*Col\s*(\d+)\](?:\s*:\s*(.+))?/g;
    let match;
    let foundStructured = false;

    while ((match = errorPattern.exec(msg)) !== null) {
      foundStructured = true;
      issues.push({
        message: match[3]?.trim() || match[0].trim(),
        line: parseInt(match[1], 10),
        column: parseInt(match[2], 10),
        severity: "error",
      });
    }

    if (!foundStructured) {
      // Parse line-by-line for simpler error formats
      for (const line of msg.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed) continue;

        const lineMatch = trimmed.match(/\[Ln\s*(\d+),\s*Col\s*(\d+)\]/);
        issues.push({
          message: trimmed,
          line: lineMatch ? parseInt(lineMatch[1], 10) : 0,
          column: lineMatch ? parseInt(lineMatch[2], 10) : 0,
          severity: "error",
        });
      }
    }
  }

  // Handle result array format
  if (json.result && Array.isArray(json.result)) {
    for (const item of json.result) {
      const r = item as Record<string, unknown>;
      issues.push({
        message: (r.message || r.error || "Unknown error") as string,
        line: (r.line as number) || 0,
        column: (r.column as number) || 0,
        severity: (r.severity as string) === "warning" ? "warning" : "error",
      });
    }
  }

  // Handle warnings array
  if (json.warnings && Array.isArray(json.warnings)) {
    for (const warn of json.warnings) {
      issues.push({
        message:
          typeof warn === "string"
            ? warn
            : ((warn as Record<string, unknown>).message as string),
        line: 0,
        column: 0,
        severity: "warning",
      });
    }
  }

  return {
    success:
      json.status === 0 &&
      issues.filter((i) => i.severity === "error").length === 0,
    issues,
  };
}
