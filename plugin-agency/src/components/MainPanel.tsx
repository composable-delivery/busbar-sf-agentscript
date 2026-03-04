import React from 'react';
import { Box, Text } from 'ink';
import { OverviewView, AgentMeta } from './OverviewView.js';
import { GraphView, GraphViewMode } from './GraphView.js';
import { ValidationView } from './ValidationView.js';
import { DepsView } from './DepsView.js';
import { TopicsView } from './TopicsView.js';
import { PathsView } from './PathsView.js';
import { GraphExport } from '../lib/logic/graph.js';
import { ValidationResult } from '../lib/logic/validation.js';
import { DepsResult } from '../lib/logic/deps.js';
import { PathsResult } from '../lib/logic/paths.js';

export type ActiveTab = 'overview' | 'graph' | 'topics' | 'deps' | 'paths' | 'validate';

export const TABS: Array<{ key: ActiveTab; label: string }> = [
  { key: 'overview', label: 'Overview' },
  { key: 'graph',    label: 'Graph'    },
  { key: 'topics',   label: 'Topics'   },
  { key: 'deps',     label: 'Deps'     },
  { key: 'paths',    label: 'Paths'    },
  { key: 'validate', label: 'Validate' },
];

interface MainPanelProps {
  activeTab: ActiveTab;
  focused: boolean;
  graphData: GraphExport | null;
  agentMeta: AgentMeta | null;
  asciiGraph: string | null;
  graphViewMode: GraphViewMode;
  validationResult: ValidationResult | null;
  depsResult: DepsResult | null;
  pathsResult: PathsResult | null;
  scrollOffset: number;
  selectedPathIdx: number;
  panelHeight: number;
  panelWidth: number;
  loading: boolean;
}

export function MainPanel({
  activeTab,
  focused,
  graphData,
  agentMeta,
  asciiGraph,
  graphViewMode,
  validationResult,
  depsResult,
  pathsResult,
  scrollOffset,
  selectedPathIdx,
  panelHeight,
  panelWidth,
  loading,
}: MainPanelProps): React.ReactElement {
  const borderColor = focused ? 'cyan' : 'gray';

  // Tab bar: 1 line. Outer border: 2 lines. Content = panelHeight - 3.
  const contentHeight = Math.max(1, panelHeight - 3);
  const innerWidth = Math.max(20, panelWidth - 2);

  return (
    <Box
      flexDirection="column"
      flexGrow={1}
      height={panelHeight}
      borderStyle="single"
      borderColor={borderColor}
      overflow="hidden"
    >
      {/* Tab bar */}
      <Box paddingX={1} flexShrink={0}>
        {TABS.map((tab, i) => {
          const isActive = activeTab === tab.key;
          return (
            <Box key={tab.key} marginRight={2}>
              <Text color="gray" dimColor={!isActive}>{i + 1} </Text>
              <Text
                color={isActive ? 'cyan' : 'gray'}
                bold={isActive}
                underline={isActive}
                dimColor={!isActive}
              >
                {tab.label}
              </Text>
            </Box>
          );
        })}
      </Box>

      {/* Content */}
      <Box height={contentHeight} overflow="hidden" flexDirection="column">
        {loading ? (
          <Box paddingX={1}><Text color="yellow">Loading…</Text></Box>
        ) : (
          <>
            {activeTab === 'overview' && (
              <OverviewView
                graphData={graphData}
                agentMeta={agentMeta}
                validationResult={validationResult}
                depsResult={depsResult}
                pathsResult={pathsResult}
                height={contentHeight}
                width={innerWidth}
                scrollOffset={scrollOffset}
              />
            )}
            {activeTab === 'graph' && (
              <GraphView
                asciiGraph={asciiGraph}
                graphData={graphData}
                viewMode={graphViewMode}
                scrollOffset={scrollOffset}
                maxLines={contentHeight - 2}
                panelWidth={innerWidth}
              />
            )}
            {activeTab === 'topics' && (
              <TopicsView
                graphData={graphData}
                scrollOffset={scrollOffset}
                maxLines={contentHeight - 1}
              />
            )}
            {activeTab === 'deps' && (
              <DepsView
                result={depsResult}
                scrollOffset={scrollOffset}
                maxLines={contentHeight - 1}
              />
            )}
            {activeTab === 'paths' && (
              <PathsView
                result={pathsResult}
                selectedPathIdx={selectedPathIdx}
                maxLines={contentHeight}
              />
            )}
            {activeTab === 'validate' && (
              <ValidationView
                result={validationResult}
                scrollOffset={scrollOffset}
                maxLines={contentHeight - 1}
              />
            )}
          </>
        )}
      </Box>
    </Box>
  );
}
