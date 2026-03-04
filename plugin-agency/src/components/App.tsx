import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Box, useInput, useApp, useStdout } from 'ink';
import { Header } from './Header.js';
import { FilePicker } from './FilePicker.js';
import { MainPanel, ActiveTab, TABS } from './MainPanel.js';
import { StatusBar } from './StatusBar.js';
import { GraphViewMode } from './GraphView.js';
import { AgentMeta } from './OverviewView.js';
import { GraphExport, getGraphData, renderAsciiGraph } from '../lib/logic/graph.js';
import { ValidationResult, validateAgent } from '../lib/logic/validation.js';
import { DepsResult, extractActionInterfaces, summarizeDependencies } from '../lib/logic/deps.js';
import { PathsResult, computePaths } from '../lib/logic/paths.js';
// @ts-ignore
import * as graphLib from '../wasm-loader.js';
import * as fs from 'fs';
import * as path from 'path';

interface AppProps {
  files: string[];
  initialFileIdx?: number;
}

const TAB_KEYS = TABS.map(t => t.key);
const FILE_PICKER_WIDTH = 34; // 32 content + 2 borders

export function App({ files, initialFileIdx = 0 }: AppProps): React.ReactElement {
  const { exit } = useApp();
  const { stdout } = useStdout();

  // ── Terminal dimensions ─────────────────────────────────────────────────
  const termRows = stdout?.rows ?? 30;
  const termCols = stdout?.columns ?? 120;

  // Layout math (fixed, no auto-grow):
  //   Header: 1 line
  //   StatusBar: 1 line
  //   Body: termRows - 2
  const bodyHeight = Math.max(6, termRows - 2);

  // File picker: bodyHeight (full height of body)
  // File picker content: bodyHeight - 2 (border) - 1 (title) - 1 (footer) = bodyHeight - 4
  const filePickerContentHeight = Math.max(2, bodyHeight - 4);

  // Main panel: bodyHeight
  // Main panel content: bodyHeight - 2 (border) - 1 (tab bar) = bodyHeight - 3
  const mainPanelHeight = bodyHeight;
  const mainPanelWidth = termCols - (files.length > 1 ? FILE_PICKER_WIDTH : 0);

  // ── File list & search ──────────────────────────────────────────────────
  const [selectedFileIdx, setSelectedFileIdx] = useState(initialFileIdx);
  const [searchMode, setSearchMode] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [filteredIdx, setFilteredIdx] = useState(0);

  const relFiles = useMemo(() => files.map(f => path.relative(process.cwd(), f)), [files]);

  const filteredRelFiles = useMemo(() => {
    if (!searchQuery) return relFiles;
    const q = searchQuery.toLowerCase();
    return relFiles.filter(f => f.toLowerCase().includes(q));
  }, [relFiles, searchQuery]);

  const filteredToOriginal = useMemo(() => {
    if (!searchQuery) return filteredRelFiles.map((_, i) => i);
    return filteredRelFiles.map(f => relFiles.indexOf(f));
  }, [filteredRelFiles, relFiles, searchQuery]);

  // ── Navigation state ────────────────────────────────────────────────────
  const [activeTab, setActiveTab] = useState<ActiveTab>('overview');
  const [focusedPane, setFocusedPane] = useState<'files' | 'main'>(
    files.length === 1 ? 'main' : 'files'
  );
  const [graphViewMode, setGraphViewMode] = useState<GraphViewMode>('topics');
  const [scrollOffset, setScrollOffset] = useState(0);
  const [selectedPathIdx, setSelectedPathIdx] = useState(0);

  // ── Data state ──────────────────────────────────────────────────────────
  const [graphData, setGraphData] = useState<GraphExport | null>(null);
  const [agentMeta, setAgentMeta] = useState<AgentMeta | null>(null);
  const [asciiGraph, setAsciiGraph] = useState<string | null>(null);
  const [validationResult, setValidationResult] = useState<ValidationResult | null>(null);
  const [depsResult, setDepsResult] = useState<DepsResult | null>(null);
  const [pathsResult, setPathsResult] = useState<PathsResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const currentAbsFile = files[selectedFileIdx] ?? null;
  const currentRelFile = currentAbsFile ? path.relative(process.cwd(), currentAbsFile) : null;

  // ── File loading ─────────────────────────────────────────────────────────
  const loadFile = useCallback(async (filePath: string, viewMode: GraphViewMode) => {
    setLoading(true);
    setError(null);
    setScrollOffset(0);
    setSelectedPathIdx(0);
    // Clear stale data immediately so UI shows loading state
    setGraphData(null);
    setAgentMeta(null);
    setAsciiGraph(null);
    setValidationResult(null);
    setDepsResult(null);
    setPathsResult(null);

    try {
      const source = await fs.promises.readFile(filePath, 'utf-8');
      const relPath = path.relative(process.cwd(), filePath);
      const fileBase = path.basename(filePath, '.agent');

      // Parse WASM once, share results
      let ast: any = null;
      try { ast = graphLib.parse_agent(source); } catch { /* ignore */ }

      // Extract AgentMeta from AST
      const meta: AgentMeta = { displayName: fileBase, variables: [] };
      if (ast) {
        try {
          const config = ast?.config?.node ?? ast?.config;
          if (config) {
            const n = config.agent_name?.node || config.agent_name?.value;
            if (n) meta.displayName = n;
            const l = config.agent_label?.node || config.agent_label?.value;
            if (l) meta.label = l;
            const d = config.description?.node || config.description?.value;
            if (d) meta.description = d;
          }
          if (ast.variables) {
            for (const [varName, varVal] of Object.entries(ast.variables as Record<string, any>)) {
              const vn = varVal?.node ?? varVal;
              const rawType = vn?.var_type?.node || vn?.var_type?.value || vn?.var_type;
              let varType = 'unknown';
              if (typeof rawType === 'string') {
                varType = rawType;
              } else if (rawType?.List) {
                varType = `List[${rawType.List}]`;
              } else if (rawType && typeof rawType === 'object') {
                varType = Object.keys(rawType)[0] ?? 'object';
              }
              meta.variables.push({
                name: varName,
                varType,
                mutable: Boolean(vn?.is_mutable),
                linked: Boolean(vn?.is_linked),
              });
            }
          }
        } catch { /* use defaults */ }
      }

      // Run analyses in parallel
      const [graph, ascii, validation] = await Promise.all([
        Promise.resolve().then(() => { try { return getGraphData(source); } catch { return null; } }),
        Promise.resolve().then(() => { try { return renderAsciiGraph(source, viewMode); } catch { return null; } }),
        Promise.resolve().then(() => { try { return validateAgent(source, relPath); } catch { return null; } }),
      ]);

      let deps: DepsResult | null = null;
      try {
        const report = graphLib.extract_dependencies(source);
        const interfaces = extractActionInterfaces(ast ?? {});
        const summary = summarizeDependencies(report);
        deps = { file: relPath, report, interfaces, summary };
      } catch { /* leave null */ }

      const paths = graph ? computePaths(graph) : null;

      setAgentMeta(meta);
      setGraphData(graph);
      setAsciiGraph(ascii);
      setValidationResult(validation);
      setDepsResult(deps);
      setPathsResult(paths);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const reloadAscii = useCallback(async (filePath: string, viewMode: GraphViewMode) => {
    try {
      const source = await fs.promises.readFile(filePath, 'utf-8');
      setAsciiGraph(renderAsciiGraph(source, viewMode));
      setScrollOffset(0);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    if (currentAbsFile) {
      loadFile(currentAbsFile, graphViewMode);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedFileIdx]);

  // ── Keyboard ────────────────────────────────────────────────────────────
  useInput((input, key) => {
    // ── Search mode captures all input ──
    if (searchMode) {
      if (key.escape || (key.ctrl && input === 'c')) {
        setSearchMode(false); setSearchQuery(''); setFilteredIdx(0); return;
      }
      if (key.return) {
        const origIdx = filteredToOriginal[filteredIdx] ?? 0;
        setSelectedFileIdx(origIdx);
        setSearchMode(false); setFocusedPane('main'); return;
      }
      if (key.backspace || key.delete) {
        setSearchQuery(q => q.slice(0, -1)); setFilteredIdx(0); return;
      }
      if (key.upArrow) { setFilteredIdx(i => Math.max(0, i - 1)); return; }
      if (key.downArrow) { setFilteredIdx(i => Math.min(filteredRelFiles.length - 1, i + 1)); return; }
      if (input && !key.ctrl && !key.meta) { setSearchQuery(q => q + input); setFilteredIdx(0); }
      return;
    }

    // ── Quit ──
    if (input === 'q' || (key.ctrl && input === 'c')) { exit(); return; }

    // ── Reload ──
    if (input === 'r' && currentAbsFile) { loadFile(currentAbsFile, graphViewMode); return; }

    // ── Number keys: always switch tabs ──
    const num = parseInt(input, 10);
    if (num >= 1 && num <= TABS.length) {
      setActiveTab(TAB_KEYS[num - 1] as ActiveTab);
      setScrollOffset(0);
      return;
    }

    // ── Tab / Shift+Tab: always cycle tabs ──
    if (key.tab && !key.shift) {
      const curr = TAB_KEYS.indexOf(activeTab);
      setActiveTab(TAB_KEYS[(curr + 1) % TAB_KEYS.length] as ActiveTab);
      setScrollOffset(0); return;
    }
    if (key.tab && key.shift) {
      const curr = TAB_KEYS.indexOf(activeTab);
      setActiveTab(TAB_KEYS[(curr - 1 + TAB_KEYS.length) % TAB_KEYS.length] as ActiveTab);
      setScrollOffset(0); return;
    }

    // ── Focus / search ──
    if (input === 'f' && files.length > 1) { setFocusedPane('files'); return; }
    if (input === '/' && files.length > 1) {
      setSearchMode(true); setSearchQuery('');
      const cur = filteredToOriginal.indexOf(selectedFileIdx);
      setFilteredIdx(cur >= 0 ? cur : 0);
      setFocusedPane('files'); return;
    }

    // ── Graph view cycle ──
    if (input === 'v' && activeTab === 'graph' && currentAbsFile) {
      const modes: GraphViewMode[] = ['topics', 'ascii'];
      const next = modes[(modes.indexOf(graphViewMode) + 1) % modes.length];
      setGraphViewMode(next);
      if (next === 'ascii') reloadAscii(currentAbsFile, 'topics');
      return;
    }

    // ── Arrow navigation ──
    if (key.upArrow) {
      if (focusedPane === 'files') {
        setSelectedFileIdx(p => Math.max(0, p - 1)); setScrollOffset(0);
      } else if (activeTab === 'paths') {
        setSelectedPathIdx(i => Math.max(0, i - 1));
      } else {
        setScrollOffset(i => Math.max(0, i - 1));
      }
      return;
    }
    if (key.downArrow) {
      if (focusedPane === 'files') {
        setSelectedFileIdx(p => Math.min(files.length - 1, p + 1)); setScrollOffset(0);
      } else if (activeTab === 'paths') {
        setSelectedPathIdx(i => Math.min((pathsResult?.total_paths ?? 1) - 1, i + 1));
      } else {
        setScrollOffset(i => i + 1);
      }
      return;
    }
    if (key.leftArrow) {
      if (activeTab === 'paths') { setSelectedPathIdx(i => Math.max(0, i - 1)); return; }
      if (focusedPane === 'main' && files.length > 1) { setFocusedPane('files'); return; }
    }
    if (key.rightArrow) {
      if (activeTab === 'paths') {
        setSelectedPathIdx(i => Math.min((pathsResult?.total_paths ?? 1) - 1, i + 1)); return;
      }
      if (focusedPane === 'files') { setFocusedPane('main'); return; }
    }
    if (key.return && focusedPane === 'files') { setFocusedPane('main'); return; }
  });

  // ── Render ──────────────────────────────────────────────────────────────
  const displayFiles = searchMode ? filteredRelFiles : relFiles;
  const displaySelectedIdx = searchMode ? filteredIdx : selectedFileIdx;

  return (
    // Root box: FIXED to terminal height — prevents layout from growing and pushing header off screen
    <Box flexDirection="column" height={termRows}>
      {/* Header — 1 line, never shrinks */}
      <Box flexShrink={0}>
        <Header
          file={currentRelFile}
          loading={loading}
          error={error}
          termCols={termCols}
          valid={validationResult?.valid ?? null}
          stats={graphData ? { topics: graphData.stats.topics, actions: graphData.stats.action_defs } : null}
        />
      </Box>

      {/* Body — fixed height, row layout */}
      <Box flexDirection="row" height={bodyHeight} overflow="hidden" flexShrink={0}>
        {/* File picker */}
        {files.length > 1 && (
          <FilePicker
            files={displayFiles}
            selectedIdx={displaySelectedIdx}
            focused={focusedPane === 'files'}
            searchMode={searchMode}
            searchQuery={searchQuery}
            maxHeight={filePickerContentHeight}
            panelHeight={bodyHeight}
          />
        )}

        {/* Main panel */}
        <MainPanel
          activeTab={activeTab}
          focused={focusedPane === 'main' || files.length === 1}
          graphData={graphData}
          agentMeta={agentMeta}
          asciiGraph={asciiGraph}
          graphViewMode={graphViewMode}
          validationResult={validationResult}
          depsResult={depsResult}
          pathsResult={pathsResult}
          scrollOffset={scrollOffset}
          selectedPathIdx={selectedPathIdx}
          panelHeight={mainPanelHeight}
          panelWidth={mainPanelWidth}
          loading={loading}
        />
      </Box>

      {/* Status bar — 1 line, never shrinks */}
      <Box flexShrink={0}>
        <StatusBar
          focusedPane={files.length === 1 ? 'main' : focusedPane}
          activeTab={activeTab}
          searchMode={searchMode}
          searchQuery={searchQuery}
        />
      </Box>
    </Box>
  );
}
