import React from 'react';
import { Box, Text } from 'ink';
import { ValidationResult } from '../lib/logic/validation.js';

interface ValidationViewProps {
  result: ValidationResult | null;
  scrollOffset: number;
  maxLines?: number;
}

export function ValidationView({ result, scrollOffset, maxLines = 30 }: ValidationViewProps): React.ReactElement {
  if (!result) {
    return (
      <Box paddingX={1}>
        <Text color="gray" dimColor>No validation data loaded</Text>
      </Box>
    );
  }

  const errorCount = result.issues.filter(i => i.severity === 'Error').length;
  const warnCount = result.issues.filter(i => i.severity === 'Warning').length;

  // Build rows so we can slice
  const rows: React.ReactElement[] = [];
  let key = 0;

  rows.push(
    <Box key={key++} marginBottom={1}>
      {result.valid ? (
        <Text color="green">✓ Valid — no issues</Text>
      ) : (
        <Text color="red">
          ✗ Invalid — {errorCount > 0 ? `${errorCount} error${errorCount === 1 ? '' : 's'}` : ''}
          {errorCount > 0 && warnCount > 0 ? ', ' : ''}
          {warnCount > 0 ? `${warnCount} warning${warnCount === 1 ? '' : 's'}` : ''}
        </Text>
      )}
    </Box>
  );

  for (const issue of result.issues) {
    rows.push(
      <Box key={key++} flexDirection="column" marginBottom={1}>
        <Box>
          <Text color={issue.severity === 'Error' ? 'red' : 'yellow'}>
            {issue.severity === 'Error' ? '✗' : '⚠'}{' '}
          </Text>
          {issue.line !== undefined && (
            <Text color="gray" dimColor>L{issue.line}:{issue.column}  </Text>
          )}
          <Text>{issue.message}</Text>
        </Box>
        {issue.hint && (
          <Text color="gray" dimColor>  ↳ {issue.hint}</Text>
        )}
      </Box>
    );
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
