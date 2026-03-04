import React from 'react';
import { Box, Text } from 'ink';
import { PathsResult, PathEntry } from '../lib/logic/paths.js';

interface PathsViewProps {
  result: PathsResult | null;
  selectedPathIdx: number;   // which path is currently focused
  maxLines: number;
}

function renderPath(path: PathEntry, highlight: boolean): React.ReactElement {
  const parts: React.ReactElement[] = [];

  for (let i = 0; i < path.nodes.length; i++) {
    const node = path.nodes[i];
    const isFirst = i === 0;

    if (!isFirst) {
      const edgeType = path.edge_types[i - 1];
      const arrow = edgeType === 'delegates' ? ' ⇒ ' : ' → ';
      parts.push(
        <Text key={`e${i}`} color={edgeType === 'delegates' ? 'blue' : 'gray'}>
          {arrow}
        </Text>
      );
    }

    parts.push(
      <Text key={`n${i}`} color={isFirst ? 'white' : highlight ? 'cyan' : 'white'} bold={isFirst || highlight}>
        {node}
      </Text>
    );
  }

  if (path.has_cycle) {
    parts.push(<Text key="cycle" color="yellow"> ↩</Text>);
  }

  return <Box key="path" flexWrap="wrap">{parts}</Box>;
}

export function PathsView({ result, selectedPathIdx, maxLines }: PathsViewProps): React.ReactElement {
  if (!result) {
    return (
      <Box paddingX={1}>
        <Text color="gray" dimColor>No path data loaded</Text>
      </Box>
    );
  }

  const { paths, unreachable, total_paths } = result;
  const cycleCount = paths.filter(p => p.has_cycle).length;

  if (total_paths === 0) {
    return (
      <Box flexDirection="column" paddingX={1}>
        <Text color="gray" dimColor>No execution paths found</Text>
        {unreachable.length > 0 && (
          <Box flexDirection="column" marginTop={1}>
            <Text color="yellow">Unreachable topics:</Text>
            {unreachable.map(t => <Text key={t} color="yellow">  ! {t}</Text>)}
          </Box>
        )}
      </Box>
    );
  }

  const currentPath = paths[selectedPathIdx] ?? paths[0];

  // Paths list: show a windowed list
  const listHeight = Math.max(4, Math.floor(maxLines * 0.4));
  const listStart = Math.max(0, selectedPathIdx - Math.floor(listHeight / 2));
  const listVisible = paths.slice(listStart, listStart + listHeight);

  // Detail lines for the selected path
  const detailLines: React.ReactElement[] = [];
  if (currentPath) {
    detailLines.push(
      <Box key="detail-header" marginBottom={1} marginTop={1}>
        <Text bold color="cyan">Path {selectedPathIdx + 1}/{total_paths}</Text>
        {currentPath.has_cycle && <Text color="yellow"> ↩ cycle</Text>}
        <Text color="gray" dimColor> · {currentPath.nodes.length} steps</Text>
      </Box>
    );
    detailLines.push(
      <Box key="detail-path" flexDirection="column" paddingX={1}>
        {renderPath(currentPath, true)}
      </Box>
    );
  }

  return (
    <Box flexDirection="column" paddingX={1}>
      {/* Summary */}
      <Box marginBottom={1}>
        <Text bold>{total_paths} paths</Text>
        {cycleCount > 0 && <Text color="yellow">  · {cycleCount} cyclic</Text>}
        {unreachable.length > 0 && <Text color="yellow">  · {unreachable.length} unreachable</Text>}
        <Text color="gray" dimColor>   ↑↓ navigate · ← → same</Text>
      </Box>

      {/* Paths list */}
      {listVisible.map((path, i) => {
        const absIdx = listStart + i;
        const isSelected = absIdx === selectedPathIdx;
        return (
          <Box key={absIdx}>
            <Text color={isSelected ? 'cyan' : 'gray'} bold={isSelected}>
              {isSelected ? '▶ ' : '  '}
            </Text>
            <Text color={isSelected ? 'white' : 'gray'} dimColor={!isSelected}>
              {path.nodes.join(' → ')}{path.has_cycle ? ' ↩' : ''}
            </Text>
          </Box>
        );
      })}

      {/* Selected path detail */}
      <Box flexDirection="column" borderStyle="single" borderColor="gray" marginTop={1} paddingX={1}>
        {detailLines}
      </Box>

      {/* Unreachable */}
      {unreachable.length > 0 && (
        <Box flexDirection="column" marginTop={1}>
          <Text color="yellow">Unreachable:</Text>
          <Box>
            {unreachable.map(t => (
              <Text key={t} color="yellow">  ! {t}</Text>
            ))}
          </Box>
        </Box>
      )}
    </Box>
  );
}
