import React from 'react';
import { Box, Text } from 'ink';

interface StatusBarProps {
  focusedPane: 'files' | 'main';
  activeTab: string;
  searchMode: boolean;
  searchQuery: string;
}

export function StatusBar({ focusedPane, activeTab, searchMode, searchQuery }: StatusBarProps): React.ReactElement {
  if (searchMode) {
    return (
      <Box>
        <Text color="cyan">/ </Text>
        <Text>{searchQuery}</Text>
        <Text color="gray" dimColor>  Esc clear · Enter confirm</Text>
      </Box>
    );
  }

  const tabHints = '1-6 tab';
  const fileHints = focusedPane === 'files'
    ? '↑↓ select · Enter open · /search'
    : 'f focus files';
  const mainHints = focusedPane === 'main'
    ? `↑↓ scroll${activeTab === 'graph' ? ' · v cycle-view' : activeTab === 'paths' ? ' · ←→ path' : ''}`
    : 'Tab→ main';

  return (
    <Box>
      <Text color="gray" dimColor>{tabHints}  │  {fileHints}  │  {mainHints}  │  r reload  │  q quit</Text>
    </Box>
  );
}
