import React from 'react';
import { Box, Text } from 'ink';
import { theme } from '../theme.js';

interface HeaderProps {
  file: string | null;
  loading: boolean;
  error: string | null;
  termCols: number;
  valid?: boolean | null;
  stats?: { topics: number; actions: number } | null;
}

/** Truncate a file path, always keeping the filename visible.
 *  e.g. "a/b/c/d/file.agent" → "…/c/d/file.agent" */
function truncatePath(p: string, max: number): string {
  if (p.length <= max) return p;
  const parts = p.split('/');
  const file = parts[parts.length - 1];
  // Walk backwards adding parts until we'd exceed max
  let result = file;
  for (let i = parts.length - 2; i >= 0; i--) {
    const candidate = `…/${parts.slice(i).join('/')}`;
    if (candidate.length <= max) result = candidate;
    else break;
  }
  return result;
}

export function Header({ file, loading, error, termCols, valid, stats }: HeaderProps): React.ReactElement {
  // Logo: fixed width  →  "☰ Agency" = 8 chars + 1 padding = 9
  const LOGO_W = theme.logoMark.length + theme.productName.length + 2;
  // Stats: " · 5t 14a ✓" ≈ 14 chars (optional)
  const STATS_W = stats ? 14 : 0;
  // Separators + padding: " · " (3) × 2 + margins
  const CHROME_W = LOGO_W + STATS_W + 8;
  const pathMax = Math.max(10, termCols - CHROME_W);

  const pathStr = file ? truncatePath(file, pathMax) : null;

  const validBadge = valid === true ? (
    <Text color={theme.success}> ✓</Text>
  ) : valid === false ? (
    <Text color={theme.error}> ✗</Text>
  ) : null;

  return (
    <Box paddingX={1}>
      {/* Logo mark + product name */}
      <Text color={theme.brand} bold>{theme.logoMark}</Text>
      <Text bold> {theme.productName}</Text>

      {/* File path */}
      {pathStr ? (
        <>
          <Text color={theme.muted} dimColor>  ·  </Text>
          {error ? (
            <Text color={theme.error}>{error}</Text>
          ) : loading ? (
            <Text color={theme.warning}>{pathStr}  loading…</Text>
          ) : (
            <Text color={theme.muted}>{pathStr}</Text>
          )}
        </>
      ) : !file && !loading ? (
        <>
          <Text color={theme.muted} dimColor>  ·  </Text>
          <Text color={theme.muted} dimColor>no file selected</Text>
        </>
      ) : null}

      {/* Stats */}
      {stats && !loading && (
        <>
          <Text color={theme.muted} dimColor>  ·  </Text>
          <Text color={theme.muted} dimColor>{stats.topics}t {stats.actions}a</Text>
          {validBadge}
        </>
      )}
    </Box>
  );
}
