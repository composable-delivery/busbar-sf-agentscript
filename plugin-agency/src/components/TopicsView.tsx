import React from 'react';
import { Box, Text } from 'ink';
import { GraphExport, TopicExportInfo, actionName } from '../lib/logic/graph.js';

interface TopicsViewProps {
  graphData: GraphExport | null;
  scrollOffset: number;
  maxLines?: number;
}

export function TopicsView({ graphData, scrollOffset, maxLines = 30 }: TopicsViewProps): React.ReactElement {
  if (!graphData) {
    return (
      <Box paddingX={1}>
        <Text color="gray" dimColor>No graph data loaded</Text>
      </Box>
    );
  }

  // Build all rows
  const rows: React.ReactElement[] = [];
  let key = 0;

  rows.push(
    <Box key={key++} marginBottom={1}>
      <Text bold>Topics </Text>
      <Text color="gray" dimColor>({graphData.topics.length} total)</Text>
    </Box>
  );

  for (const topic of graphData.topics) {
    // Topic header
    rows.push(
      <Box key={key++} marginTop={1}>
        <Text color={topic.is_entry ? 'white' : 'cyan'} bold>
          {topic.is_entry ? '★ ' : '■ '}{topic.name}
          {topic.is_entry ? ' (entry)' : ''}
        </Text>
      </Box>
    );
    // Description
    if (topic.description) {
      rows.push(
        <Box key={key++}>
          <Text color="gray" dimColor>  {topic.description}</Text>
        </Box>
      );
    }
    // Actions
    if (topic.actions && topic.actions.length > 0) {
      rows.push(
        <Box key={key++}>
          <Text color="magenta" dimColor>
            {'  '}Actions: {topic.actions.map(actionName).join(', ')}
          </Text>
        </Box>
      );
    }
    // Transitions
    if (topic.transitions_to.length > 0) {
      rows.push(
        <Box key={key++}>
          <Text color="cyan" dimColor>
            {'  '}→ {topic.transitions_to.join(', ')}
          </Text>
        </Box>
      );
    }
    // Delegates
    if (topic.delegates_to.length > 0) {
      rows.push(
        <Box key={key++}>
          <Text color="blue" dimColor>
            {'  '}⇒ {topic.delegates_to.join(', ')}
          </Text>
        </Box>
      );
    }
    // Terminal
    if (topic.transitions_to.length === 0 && topic.delegates_to.length === 0) {
      rows.push(
        <Box key={key++}>
          <Text color="gray" dimColor>  (terminal)</Text>
        </Box>
      );
    }
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
