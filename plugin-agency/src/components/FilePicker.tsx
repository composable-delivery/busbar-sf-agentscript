import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import * as path from 'path';

const PANE_WIDTH = 32;

function truncate(s: string, max: number): string {
  return s.length > max ? s.slice(0, max - 1) + '…' : s;
}

/** Return a display name for each file. Uses basename without extension.
 *  If two files share the same basename, disambiguate with the parent directory. */
function makeDisplayNames(relFiles: string[]): string[] {
  const basenames = relFiles.map(f => path.basename(f, path.extname(f)));
  const count = new Map<string, number>();
  for (const b of basenames) count.set(b, (count.get(b) ?? 0) + 1);

  return relFiles.map((f, i) => {
    const base = basenames[i];
    if ((count.get(base) ?? 1) > 1) {
      const parts = f.split('/');
      const parent = parts.length > 1 ? parts[parts.length - 2] : '';
      return parent ? `${parent}/${base}` : base;
    }
    return base;
  });
}

interface FilePickerProps {
  files: string[];        // relative paths from cwd
  selectedIdx: number;
  focused: boolean;
  searchMode: boolean;
  searchQuery: string;
  maxHeight: number;
  panelHeight: number;
}

export function FilePicker({
  files,
  selectedIdx,
  focused,
  searchMode,
  searchQuery,
  maxHeight,
  panelHeight,
}: FilePickerProps): React.ReactElement {
  const borderColor = focused ? 'cyan' : 'gray';
  const nameMax = PANE_WIDTH - 4; // 4 = border(1) + padding(1) + selector(2)

  const displayNames = useMemo(() => makeDisplayNames(files), [files]);

  // Center the selected file in the visible window
  const half = Math.floor(maxHeight / 2);
  const start = Math.max(0, Math.min(selectedIdx - half, files.length - maxHeight));
  const visibleFiles = files.slice(start, start + maxHeight);

  const title = searchMode ? `/${searchQuery}` : `Files (${files.length})`;

  return (
    <Box
      flexDirection="column"
      width={PANE_WIDTH}
      height={panelHeight}
      borderStyle="single"
      borderColor={borderColor}
      paddingX={1}
      flexShrink={0}
      overflow="hidden"
    >
      <Text bold color={borderColor}>{title}</Text>

      {files.length === 0 ? (
        <Text color="gray" dimColor>No .agent files</Text>
      ) : (
        visibleFiles.map((_, i) => {
          const idx = start + i;
          const isSelected = idx === selectedIdx;
          const label = truncate(displayNames[idx], nameMax);
          return (
            <Text
              key={idx}
              color={isSelected ? 'cyan' : 'white'}
              bold={isSelected}
              dimColor={!isSelected}
            >
              {isSelected ? '▶ ' : '  '}{label}
            </Text>
          );
        })
      )}

      {files.length > maxHeight && (
        <Text color="gray" dimColor>
          {'  '}{start + 1}–{Math.min(start + maxHeight, files.length)}/{files.length}
        </Text>
      )}
    </Box>
  );
}
