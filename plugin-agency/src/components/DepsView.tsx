import React from 'react';
import { Box, Text } from 'ink';
import { DepsResult, ActionInterface } from '../lib/logic/deps.js';

interface DepsViewProps {
  result: DepsResult | null;
  scrollOffset: number;
  maxLines?: number;
}

const TYPE_COLOR: Record<string, string> = {
  flow: 'cyanBright',
  apex: 'magenta',
  prompt: 'greenBright',
  unknown: 'white',
};

export function DepsView({ result, scrollOffset, maxLines = 30 }: DepsViewProps): React.ReactElement {
  if (!result) {
    return (
      <Box paddingX={1}>
        <Text color="gray" dimColor>No dependency data loaded</Text>
      </Box>
    );
  }

  const { summary, report, interfaces } = result;

  // Build all rows
  const rows: React.ReactElement[] = [];
  let key = 0;

  // Summary header
  rows.push(
    <Box key={key++} marginBottom={1}>
      <Text bold>Dependencies </Text>
      <Text color="gray" dimColor>({summary.total} total)</Text>
    </Box>
  );

  // Action interfaces (flows, apex, prompts) — detailed section
  const flowInterfaces = interfaces.filter(i => i.targetType === 'flow');
  const apexInterfaces = interfaces.filter(i => i.targetType === 'apex');
  const promptInterfaces = interfaces.filter(i => i.targetType === 'prompt');

  if (flowInterfaces.length > 0) {
    rows.push(<Box key={key++}><Text bold color="cyanBright">Flows ({flowInterfaces.length})</Text></Box>);
    for (const iface of flowInterfaces) {
      rows.push(<Box key={key++} flexDirection="column">
        <Text color="cyanBright">  ▸ {iface.targetName}</Text>
      </Box>);
      if (iface.description) {
        rows.push(<Box key={key++}><Text color="gray" dimColor>    {iface.description}</Text></Box>);
      }
      if (iface.inputs.length > 0) {
        rows.push(<Box key={key++}><Text color="green" dimColor>    In: {iface.inputs.map(p => `${p.name}:${p.type}${p.isRequired ? '*' : ''}`).join(', ')}</Text></Box>);
      }
      if (iface.outputs.length > 0) {
        rows.push(<Box key={key++}><Text color="yellow" dimColor>    Out: {iface.outputs.map(p => `${p.name}:${p.type}`).join(', ')}</Text></Box>);
      }
    }
  }

  if (apexInterfaces.length > 0) {
    rows.push(<Box key={key++}><Text bold color="magenta">Apex Classes ({apexInterfaces.length})</Text></Box>);
    for (const iface of apexInterfaces) {
      rows.push(<Box key={key++}><Text color="magenta">  ▸ {iface.targetName}</Text></Box>);
      if (iface.inputs.length > 0) {
        rows.push(<Box key={key++}><Text color="green" dimColor>    In: {iface.inputs.map(p => `${p.name}:${p.type}${p.isRequired ? '*' : ''}`).join(', ')}</Text></Box>);
      }
      if (iface.outputs.length > 0) {
        rows.push(<Box key={key++}><Text color="yellow" dimColor>    Out: {iface.outputs.map(p => `${p.name}:${p.type}`).join(', ')}</Text></Box>);
      }
    }
  }

  if (promptInterfaces.length > 0) {
    rows.push(<Box key={key++}><Text bold color="greenBright">Prompt Templates ({promptInterfaces.length})</Text></Box>);
    for (const iface of promptInterfaces) {
      rows.push(<Box key={key++}><Text color="greenBright">  ▸ {iface.targetName}</Text></Box>);
    }
  }

  // Simple list categories
  const simpleCats = [
    { label: 'SObjects', color: 'cyan' as const, items: report.sobjects },
    { label: 'Fields', color: 'blue' as const, items: report.fields },
    { label: 'Knowledge Bases', color: 'yellow' as const, items: report.knowledge_bases },
    { label: 'Connections', color: 'green' as const, items: report.connections },
    { label: 'External Services', color: 'red' as const, items: report.external_services },
  ].filter(c => c.items.length > 0);

  for (const cat of simpleCats) {
    rows.push(<Box key={key++}><Text bold color={cat.color}>{cat.label} ({cat.items.length})</Text></Box>);
    for (const item of cat.items) {
      rows.push(<Box key={key++}><Text color={cat.color} dimColor>  • {item}</Text></Box>);
    }
  }

  if (summary.total === 0) {
    rows.push(<Box key={key++}><Text color="gray" dimColor>  No dependencies found</Text></Box>);
  }

  const visible = rows.slice(scrollOffset, scrollOffset + maxLines);
  const canScroll = rows.length > maxLines;

  return (
    <Box flexDirection="column" paddingX={1}>
      {visible}
      {canScroll && (
        <Text color="gray" dimColor>
          ↑↓ scroll · {scrollOffset + 1}–{Math.min(scrollOffset + maxLines, rows.length)}/{rows.length}
        </Text>
      )}
    </Box>
  );
}
