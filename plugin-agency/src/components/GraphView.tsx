import React from 'react';
import { Box, Text } from 'ink';
import { GraphExport, actionName, buildTopicVariableMap } from '../lib/logic/graph.js';
import { theme } from '../theme.js';

export type GraphViewMode = 'topics' | 'ascii';

interface GraphViewProps {
  asciiGraph: string | null;
  graphData: GraphExport | null;
  viewMode: GraphViewMode;
  scrollOffset: number;
  maxLines: number;
  panelWidth: number;
}

const FLOW_COL_WIDTH = 32;

function targetParts(target: string): { kind: string; name: string } {
  if (target.startsWith('flow://')) return { kind: 'flow', name: target.slice(7) };
  if (target.startsWith('apex://')) return { kind: 'apex', name: target.slice(7) };
  if (target.startsWith('prompt://')) return { kind: 'prompt', name: target.slice(9) };
  return { kind: '', name: target };
}

const KIND_COLORS: Record<string, string> = {
  flow: theme.actionFlow,
  apex: theme.actionApex,
  prompt: theme.actionPrompt,
};

function trunc(s: string, max: number): string {
  return s.length > max ? s.slice(0, max - 1) + '…' : s;
}

// ─── Left column: ASCII flow ──────────────────────────────────────────────────
function FlowColumn({ asciiGraph, graphData }: { asciiGraph: string | null; graphData: GraphExport }): React.ReactElement {
  const lines = asciiGraph
    ? asciiGraph.split('\n').filter(l => l.trim() !== '')
    : graphData.topics.map(t => (t.is_entry ? `★ ${t.name}` : `  ${t.name}`));

  return (
    <Box flexDirection="column" width={FLOW_COL_WIDTH} flexShrink={0}>
      <Text bold color="white">Topic Flow</Text>
      {lines.map((line, i) => {
        const isEntry = line.includes('★') || (!line.startsWith(' ') && !line.startsWith('└') && !line.startsWith('│'));
        const isBranch = line.includes('└') || line.includes('│');
        const isCycle = line.includes('↩');
        return (
          <Text
            key={i}
            color={isCycle ? 'yellow' : isBranch ? 'gray' : isEntry ? 'white' : 'cyan'}
            bold={isEntry && !isBranch}
            dimColor={isBranch}
          >
            {trunc(line, FLOW_COL_WIDTH - 1)}
          </Text>
        );
      })}
    </Box>
  );
}

// ─── Right column: topic detail cards ────────────────────────────────────────
function buildDetailRows(graphData: GraphExport, colWidth: number): React.ReactElement[] {
  const rows: React.ReactElement[] = [];
  let k = 0;
  const varMap = buildTopicVariableMap(graphData.nodes);

  for (const topic of graphData.topics) {
    const reads = varMap.reads.get(topic.name);
    const writes = varMap.writes.get(topic.name);
    const actions = topic.actions ?? [];

    // Topic name
    rows.push(
      <Box key={k++} marginTop={k > 1 ? 1 : 0}>
        <Text color={topic.is_entry ? 'white' : 'cyan'} bold>
          {topic.is_entry ? '★ ' : '■ '}{topic.name}
        </Text>
        {topic.is_entry && <Text color="gray" dimColor>  entry</Text>}
        {topic.transitions_to.length === 0 && topic.delegates_to.length === 0 && (
          <Text color="gray" dimColor>  terminal</Text>
        )}
      </Box>
    );

    if (topic.description) {
      rows.push(
        <Box key={k++}>
          <Text color="gray" dimColor>  {trunc(topic.description, colWidth - 4)}</Text>
        </Box>
      );
    }

    // Var reads/writes on one line each
    if (reads && reads.size > 0) {
      rows.push(
        <Box key={k++}>
          <Text color={theme.varRead} dimColor>  reads  </Text>
          <Text color={theme.varRead}>{trunc([...reads].join(', '), colWidth - 10)}</Text>
        </Box>
      );
    }
    if (writes && writes.size > 0) {
      rows.push(
        <Box key={k++}>
          <Text color={theme.varWrite} dimColor>  writes </Text>
          <Text color={theme.varWrite}>{trunc([...writes].join(', '), colWidth - 10)}</Text>
        </Box>
      );
    }

    // Actions
    if (actions.length > 0) {
      for (const action of actions) {
        const name = actionName(action);
        const target = typeof action === 'object' ? (action.target ?? '') : '';
        if (target) {
          const { kind, name: tname } = targetParts(target);
          const color = KIND_COLORS[kind] ?? 'white';
          rows.push(
            <Box key={k++}>
              <Text color="magenta">  • {trunc(name, 22)}</Text>
              <Text color="gray" dimColor>  </Text>
              {kind && <Text color={color} dimColor>{kind}://</Text>}
              <Text color={color}>{trunc(tname, colWidth - name.length - 14)}</Text>
            </Box>
          );
        } else {
          rows.push(
            <Box key={k++}>
              <Text color="magenta">  • {name}</Text>
            </Box>
          );
        }
      }
    } else if (!topic.description && !reads && !writes) {
      rows.push(
        <Box key={k++}>
          <Text color="gray" dimColor>  (no actions)</Text>
        </Box>
      );
    }
  }

  return rows;
}

// ─── Ascii view: raw WASM output ─────────────────────────────────────────────
function buildAsciiRows(asciiGraph: string): React.ReactElement[] {
  return asciiGraph.split('\n').map((line, i) => {
    let color = 'white';
    let dim = false;
    if (/^(VARIABLES|ENTRY POINT|TOPICS):/.test(line)) color = 'white';
    else if (line.includes('┌─')) color = 'cyan';
    else if (line.includes('• ')) color = 'magenta';
    else if (line.includes('◆ ')) color = 'green';
    else if (line.includes('Transitions →')) color = 'cyan';
    else if (line.includes('Delegates ⇒')) color = 'blue';
    else if (/^[│└┌─\s]+$/.test(line)) dim = true;
    return <Box key={i}><Text color={color} dimColor={dim}>{line}</Text></Box>;
  });
}

// ─── Main ─────────────────────────────────────────────────────────────────────
export function GraphView({ asciiGraph, graphData, viewMode, scrollOffset, maxLines, panelWidth }: GraphViewProps): React.ReactElement {
  const statsLine = graphData
    ? `${graphData.stats.topics} topics · ${graphData.stats.variables} vars · ${graphData.stats.action_defs} actions · ${graphData.stats.reasoning_actions} reasoning`
    : null;

  if (!graphData) {
    return (
      <Box flexDirection="column" paddingX={1}>
        <Text color="gray" dimColor>No graph data</Text>
      </Box>
    );
  }

  if (viewMode === 'ascii') {
    const rows = asciiGraph ? buildAsciiRows(asciiGraph) : [];
    const visible = rows.slice(scrollOffset, scrollOffset + maxLines);
    return (
      <Box flexDirection="column" paddingX={1}>
        <Box marginBottom={1}>
          {statsLine && <Text color="gray" dimColor>{statsLine}   </Text>}
          <Text color="gray" dimColor>[v] ascii</Text>
        </Box>
        {visible}
        {rows.length > maxLines && (
          <Text color="gray" dimColor>↑↓ {scrollOffset + 1}–{Math.min(scrollOffset + maxLines, rows.length)}/{rows.length}</Text>
        )}
      </Box>
    );
  }

  // ── topics mode: 2-column layout ─────────────────────────────────────────
  const detailColWidth = Math.max(20, panelWidth - FLOW_COL_WIDTH - 4);
  const detailRows = buildDetailRows(graphData, detailColWidth);
  const visibleDetail = detailRows.slice(scrollOffset, scrollOffset + maxLines);
  const canScroll = detailRows.length > maxLines;

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Stats header */}
      <Box marginBottom={1}>
        {statsLine && <Text color="gray" dimColor>{statsLine}   </Text>}
        <Text color="gray" dimColor>[v] topics</Text>
      </Box>

      {/* Two-column body */}
      <Box flexDirection="row">
        {/* Left: full flow, no scrolling (always short) */}
        <FlowColumn asciiGraph={asciiGraph} graphData={graphData} />

        {/* Vertical divider */}
        <Box marginX={1}>
          <Text color="gray" dimColor>│</Text>
        </Box>

        {/* Right: action/var details, scrollable */}
        <Box flexDirection="column" width={detailColWidth}>
          {visibleDetail}
          {canScroll && (
            <Text color="gray" dimColor>
              ↑↓ {scrollOffset + 1}–{Math.min(scrollOffset + maxLines, detailRows.length)}/{detailRows.length}
            </Text>
          )}
        </Box>
      </Box>
    </Box>
  );
}
