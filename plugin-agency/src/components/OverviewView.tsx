import React from 'react';
import { Box, Text } from 'ink';
import { GraphExport, actionName } from '../lib/logic/graph.js';
import { ValidationResult } from '../lib/logic/validation.js';
import { DepsResult } from '../lib/logic/deps.js';
import { PathsResult } from '../lib/logic/paths.js';

export interface AgentMeta {
  displayName: string;
  label?: string;
  description?: string;
  variables: VariableDetail[];
}

export interface VariableDetail {
  name: string;
  varType: string;
  mutable: boolean;
  linked: boolean;
}

interface OverviewViewProps {
  graphData: GraphExport | null;
  agentMeta: AgentMeta | null;
  validationResult: ValidationResult | null;
  depsResult: DepsResult | null;
  pathsResult: PathsResult | null;
  height: number;
  width: number;
  scrollOffset: number;
}

function trunc(s: string, max: number): string {
  return s.length > max ? s.slice(0, max - 1) + '…' : s;
}

function pad(s: string, width: number): string {
  return s.length >= width ? s : s + ' '.repeat(width - s.length);
}

// ─── Topics column ────────────────────────────────────────────────────────────
function TopicsColumn({ graphData, colWidth }: { graphData: GraphExport; colWidth: number }): React.ReactElement {
  const topics = graphData.topics;
  return (
    <Box flexDirection="column" width={colWidth}>
      <Text bold color="cyan">Topics ({topics.length})</Text>
      {topics.map(t => {
        const prefix = t.is_entry ? '★ ' : '■ ';
        const nameMax = colWidth - 4;
        const name = trunc(t.name, nameMax);
        const actionTag = t.actions?.length ? ` (${t.actions.length}a)` : '';
        return (
          <Box key={t.name} flexDirection="column">
            <Text color={t.is_entry ? 'white' : 'cyan'} bold={t.is_entry}>
              {prefix}{trunc(name + actionTag, nameMax + 4)}
            </Text>
            {t.transitions_to.length > 0 && (
              <Text color="gray" dimColor>
                {'  '}→ {trunc(t.transitions_to.join(', '), colWidth - 5)}
              </Text>
            )}
            {t.delegates_to.length > 0 && (
              <Text color="blue" dimColor>
                {'  '}⇒ {trunc(t.delegates_to.join(', '), colWidth - 5)}
              </Text>
            )}
            {t.actions && t.actions.length > 0 && t.actions.slice(0, 3).map((a, ai) => {
              const an = actionName(a);
              return (
                <Text key={`${t.name}-a${ai}`} color="magenta" dimColor>
                  {'  '}• {trunc(an, colWidth - 5)}
                </Text>
              );
            })}
            {t.actions && t.actions.length > 3 && (
              <Text color="magenta" dimColor>
                {'  '}  …+{t.actions.length - 3} more
              </Text>
            )}
          </Box>
        );
      })}
      {topics.length === 0 && <Text color="gray" dimColor>  No topics</Text>}
    </Box>
  );
}

// ─── Variables column ─────────────────────────────────────────────────────────
function VarsColumn({ variables, graphVars, colWidth }: {
  variables: VariableDetail[];
  graphVars: string[];
  colWidth: number;
}): React.ReactElement {
  // Merge: use AgentMeta variables if available, else just names from graphData
  const vars = variables.length > 0 ? variables : graphVars.map(n => ({
    name: n, varType: '?', mutable: false, linked: false,
  }));

  const nameW = Math.floor(colWidth * 0.4);
  const typeW = Math.floor(colWidth * 0.35);

  return (
    <Box flexDirection="column" width={colWidth}>
      <Text bold color="cyan">Variables ({vars.length})</Text>
      {vars.map(v => {
        const mods = [v.mutable ? '●mut' : '', v.linked ? '⊛lnk' : ''].filter(Boolean).join(' ');
        return (
          <Box key={v.name}>
            <Text color="green">{pad(trunc(v.name, nameW), nameW)}</Text>
            <Text color="yellow"> {pad(trunc(v.varType, typeW), typeW)}</Text>
            {mods && <Text color="gray" dimColor> {mods}</Text>}
          </Box>
        );
      })}
      {vars.length === 0 && <Text color="gray" dimColor>  No variables</Text>}
    </Box>
  );
}

// ─── Deps column ──────────────────────────────────────────────────────────────
function DepsColumn({ depsResult, colWidth }: { depsResult: DepsResult; colWidth: number }): React.ReactElement {
  const { summary, report } = depsResult;
  const cats = [
    { key: 'flows', label: 'Flows', color: 'cyanBright' as const, items: report.flows },
    { key: 'apex_classes', label: 'Apex', color: 'magenta' as const, items: report.apex_classes },
    { key: 'sobjects', label: 'SObjects', color: 'cyan' as const, items: report.sobjects },
    { key: 'knowledge_bases', label: 'Knowledge', color: 'yellow' as const, items: report.knowledge_bases },
    { key: 'connections', label: 'Connections', color: 'green' as const, items: report.connections },
    { key: 'prompt_templates', label: 'Prompts', color: 'greenBright' as const, items: report.prompt_templates },
    { key: 'external_services', label: 'External', color: 'red' as const, items: report.external_services },
    { key: 'fields', label: 'Fields', color: 'blue' as const, items: report.fields },
  ].filter(c => c.items.length > 0);

  return (
    <Box flexDirection="column" width={colWidth}>
      <Text bold color="cyan">Dependencies ({summary.total})</Text>
      {cats.length === 0 && <Text color="gray" dimColor>  None found</Text>}
      {cats.map(c => (
        <Box key={c.key} flexDirection="column">
          <Box>
            <Text color={c.color} bold>{c.label} </Text>
            <Text color="gray" dimColor>({c.items.length})</Text>
          </Box>
          {c.items.slice(0, 3).map(item => (
            <Text key={item} color="white" dimColor>
              {'  '}{trunc(item, colWidth - 4)}
            </Text>
          ))}
          {c.items.length > 3 && (
            <Text color="gray" dimColor>{'  '}…+{c.items.length - 3} more</Text>
          )}
        </Box>
      ))}
    </Box>
  );
}

// ─── Paths column ─────────────────────────────────────────────────────────────
function PathsColumn({ pathsResult, colWidth }: { pathsResult: PathsResult; colWidth: number }): React.ReactElement {
  const cycleCount = pathsResult.paths.filter(p => p.has_cycle).length;
  const maxLen = pathsResult.paths.reduce((m, p) => Math.max(m, p.nodes.length), 0);

  return (
    <Box flexDirection="column" width={colWidth}>
      <Text bold color="cyan">Execution Paths</Text>
      <Box>
        <Text bold color="white">{pathsResult.total_paths} </Text>
        <Text color="gray" dimColor>paths</Text>
      </Box>
      {cycleCount > 0 && (
        <Box>
          <Text color="yellow">↩ {cycleCount} </Text>
          <Text color="gray" dimColor>cyclic</Text>
        </Box>
      )}
      {pathsResult.unreachable.length > 0 && (
        <Box>
          <Text color="yellow">! {pathsResult.unreachable.length} </Text>
          <Text color="gray" dimColor>unreachable</Text>
        </Box>
      )}
      {maxLen > 0 && (
        <Box>
          <Text color="gray" dimColor>longest: </Text>
          <Text>{maxLen} hops</Text>
        </Box>
      )}
      <Box marginTop={1}>
        <Text color="gray" dimColor italic>Tab 5 for details</Text>
      </Box>
    </Box>
  );
}

// ─── Separator ────────────────────────────────────────────────────────────────
function Sep({ width }: { width: number }): React.ReactElement {
  return <Text color="gray" dimColor>{'─'.repeat(Math.max(1, width))}</Text>;
}

// ─── Main ────────────────────────────────────────────────────────────────────
export function OverviewView({
  graphData,
  agentMeta,
  validationResult,
  depsResult,
  pathsResult,
  height,
  width,
  scrollOffset,
}: OverviewViewProps): React.ReactElement {
  if (!graphData && !agentMeta) {
    return (
      <Box paddingX={1}>
        <Text color="gray" dimColor>No data loaded — select a file</Text>
      </Box>
    );
  }

  const leftW = Math.floor(width / 2) - 1;
  const rightW = width - leftW - 1; // 1 for divider space

  // Build all "lines" as JSX rows so we can slice for scroll
  const rows: React.ReactElement[] = [];
  let key = 0;

  // ── Agent header row ──
  const name = agentMeta?.displayName ?? '';
  const label = agentMeta?.label ?? '';
  const desc = agentMeta?.description ?? '';
  const validBadge = validationResult
    ? validationResult.valid
      ? <Text color="green"> ✓ valid</Text>
      : <Text color="red"> ✗ {validationResult.issues.filter(i => i.severity === 'Error').length}err {validationResult.issues.filter(i => i.severity === 'Warning').length}warn</Text>
    : null;

  rows.push(
    <Box key={key++}>
      <Text bold color="white">{trunc(name, leftW)}</Text>
      {label && <Text color="gray" dimColor> · {trunc(label, 30)}</Text>}
      {validBadge}
    </Box>
  );

  if (desc) {
    rows.push(
      <Box key={key++}>
        <Text color="gray" dimColor>{trunc(desc, width)}</Text>
      </Box>
    );
  }

  // ── Stats row ──
  if (graphData) {
    const s = graphData.stats;
    rows.push(
      <Box key={key++}>
        <Text color="gray" dimColor>
          {s.topics} topics · {s.variables} vars · {s.action_defs} actions · {s.reasoning_actions} reasoning
        </Text>
      </Box>
    );
  }

  rows.push(<Sep key={key++} width={width} />);

  // ── Topics + Variables side by side ──
  if (graphData) {
    const variables = agentMeta?.variables ?? [];
    const graphVars = graphData.variables;

    rows.push(
      <Box key={key++} flexDirection="row">
        <TopicsColumn graphData={graphData} colWidth={leftW} />
        <Text color="gray" dimColor> │</Text>
        <Box width={1} />
        <VarsColumn variables={variables} graphVars={graphVars} colWidth={rightW - 2} />
      </Box>
    );
  }

  rows.push(<Sep key={key++} width={width} />);

  // ── Deps + Paths side by side ──
  if (depsResult || pathsResult) {
    rows.push(
      <Box key={key++} flexDirection="row">
        {depsResult ? (
          <DepsColumn depsResult={depsResult} colWidth={leftW} />
        ) : (
          <Box width={leftW}><Text color="gray" dimColor>No dep data</Text></Box>
        )}
        <Text color="gray" dimColor> │</Text>
        <Box width={1} />
        {pathsResult ? (
          <PathsColumn pathsResult={pathsResult} colWidth={rightW - 2} />
        ) : (
          <Box width={rightW - 2}><Text color="gray" dimColor>No path data</Text></Box>
        )}
      </Box>
    );
    rows.push(<Sep key={key++} width={width} />);
  }

  // ── Validation detail (if issues) ──
  if (validationResult && validationResult.issues.length > 0) {
    const errCount = validationResult.issues.filter(i => i.severity === 'Error').length;
    const warnCount = validationResult.issues.filter(i => i.severity === 'Warning').length;
    rows.push(
      <Box key={key++}>
        <Text bold color="cyan">Validation </Text>
        {errCount > 0 && <Text color="red">{errCount} error{errCount !== 1 ? 's' : ''} </Text>}
        {warnCount > 0 && <Text color="yellow">{warnCount} warning{warnCount !== 1 ? 's' : ''}</Text>}
      </Box>
    );
    for (const issue of validationResult.issues.slice(0, 8)) {
      rows.push(
        <Box key={key++} flexDirection="column">
          <Box>
            <Text color={issue.severity === 'Error' ? 'red' : 'yellow'}>
              {issue.severity === 'Error' ? '✗' : '⚠'}{' '}
            </Text>
            {issue.line !== undefined && (
              <Text color="gray" dimColor>L{issue.line}  </Text>
            )}
            <Text>{trunc(issue.message, width - 12)}</Text>
          </Box>
          {issue.hint && (
            <Text color="gray" dimColor>   ↳ {trunc(issue.hint, width - 6)}</Text>
          )}
        </Box>
      );
    }
    if (validationResult.issues.length > 8) {
      rows.push(
        <Box key={key++}>
          <Text color="gray" dimColor>  …+{validationResult.issues.length - 8} more — Tab 6 for full list</Text>
        </Box>
      );
    }
  }

  // Slice for virtual scroll
  const visible = rows.slice(scrollOffset, scrollOffset + height);
  const canScroll = rows.length > height;

  return (
    <Box flexDirection="column" paddingX={1}>
      {visible}
      {canScroll && (
        <Text color="gray" dimColor>
          ↑↓ {scrollOffset + 1}–{Math.min(scrollOffset + height, rows.length)}/{rows.length}
        </Text>
      )}
    </Box>
  );
}
