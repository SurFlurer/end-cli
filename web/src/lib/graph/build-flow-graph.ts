import type { Edge, Node, Position } from '@xyflow/svelte';
import type { LogisticsGraphDto, LogisticsNodeDto } from '../types';
import { expandExternalSupplyNodes, positionLightweightNodes } from './external-supply';
import { buildFlowGraphHighlightIndex } from './highlight';
import { layoutNodesWithDagre } from './layout';
import { findSCCs } from './scc';
import {
  LIGHTWEIGHT_NODE_HEIGHT,
  LIGHTWEIGHT_NODE_WIDTH,
  NODE_HEIGHT,
  NODE_WIDTH,
  NODE_X_OFFSET,
  NODE_Y_OFFSET,
  SCC_MIN_SIZE,
  type BuildFlowGraphResult,
  type LayoutNode,
  type SCCResult
} from './types';

const kindColor: Record<string, string> = {
  external_supply: '#2f8f83',
  external_consumption: '#c65a3a',
  recipe_group: '#2a678a',
  outpost_sale: '#b16d00',
  thermal_bank_group: '#8b305e',
  warehouse_stockpile: '#5a6b7e'
};

function pickNodeColor(kind: string): string {
  return kindColor[kind] ?? '#3f5165';
}

function compareNodesByLabel(lhs: { label: string; id: string }, rhs: { label: string; id: string }): number {
  return lhs.label.localeCompare(rhs.label) || lhs.id.localeCompare(rhs.id);
}

type FlowEdge = LogisticsGraphDto['edges'][number];

function compareEdgesForLayout(
  lhs: Pick<FlowEdge, 'source' | 'target' | 'itemKey' | 'id'>,
  rhs: Pick<FlowEdge, 'source' | 'target' | 'itemKey' | 'id'>
): number {
  return (
    lhs.source.localeCompare(rhs.source) ||
    lhs.target.localeCompare(rhs.target) ||
    lhs.itemKey.localeCompare(rhs.itemKey) ||
    lhs.id.localeCompare(rhs.id)
  );
}

function createLightweightNode(node: LogisticsNodeDto): Node {
  const color = pickNodeColor(node.kind);
  return {
    id: node.id,
    type: 'input',
    data: { label: '' },
    position: { x: 0, y: 0 },
    sourcePosition: 'right' as Position,
    style:
      `width:${LIGHTWEIGHT_NODE_WIDTH}px;` +
      `height:${LIGHTWEIGHT_NODE_HEIGHT}px;` +
      `background:${color};` +
      'border-radius:50%;' +
      'border:none;' +
      'box-shadow:none;' +
      'min-width:0;' +
      'padding:0;'
  };
}

function createNormalNode(node: LogisticsNodeDto, isInScc: boolean): Node {
  const color = pickNodeColor(node.kind);

  const isOutputNode =
    node.kind === 'outpost_sale' ||
    node.kind === 'external_consumption' ||
    node.kind === 'thermal_bank_group' ||
    node.kind === 'warehouse_stockpile';

  const borderStyle = isInScc
    ? `border:2px solid ${color};box-shadow:0 0 0 1px ${color}40,0 8px 18px -16px rgba(0,0,0,0.65);`
    : `border:1px solid ${color};box-shadow:0 8px 18px -16px rgba(0,0,0,0.65);`;

  return {
    id: node.id,
    type: isOutputNode ? 'output' : undefined,
    data: {
      label: node.label,
      isInSCC: isInScc
    },
    position: { x: 0, y: 0 },
    sourcePosition: 'right' as Position,
    targetPosition: 'left' as Position,
    style:
      borderStyle +
      'border-radius:12px;' +
      'background:#fff;' +
      'color:#16313d;' +
      'min-width:220px;' +
      'font-size:12px;'
  };
}

function buildFilteredSccResult(sccResult: SCCResult, normalNodeIds: ReadonlySet<string>): SCCResult {
  const components = sccResult.components
    .map((component) => component.filter((id) => normalNodeIds.has(id)))
    .filter((component) => component.length >= SCC_MIN_SIZE);

  const nodeToComponent = new Map<string, number>();
  for (let i = 0; i < components.length; i++) {
    for (const nodeId of components[i]) {
      nodeToComponent.set(nodeId, i);
    }
  }

  return {
    components,
    nodeToComponent,
    condensedEdges: new Set(sccResult.condensedEdges)
  };
}

export function buildFlowGraph(graph: LogisticsGraphDto): BuildFlowGraphResult {
  const expanded = expandExternalSupplyNodes(graph.nodes, graph.edges);

  const nodeIdSet = new Set<string>();
  for (const edge of expanded.edges) {
    nodeIdSet.add(edge.source);
    nodeIdSet.add(edge.target);
  }

  const renderNodes = expanded.nodes.filter((node) => nodeIdSet.has(node.id)).sort(compareNodesByLabel);

  const sccResult = findSCCs(
    renderNodes.map((node) => node.id),
    expanded.edges.map((edge) => ({ source: edge.source, target: edge.target }))
  );

  const normalLayoutNodes: LayoutNode[] = [];
  const lightweightBaseNodes: Node[] = [];
  const normalBaseNodes: Node[] = [];
  const normalNodeIds = new Set<string>();

  for (const node of renderNodes) {
    const isLightweight = expanded.lightweightNodeIds.has(node.id);

    if (isLightweight) {
      lightweightBaseNodes.push(createLightweightNode(node));
      continue;
    }

    normalLayoutNodes.push({
      id: node.id,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      xOffset: NODE_X_OFFSET,
      yOffset: NODE_Y_OFFSET
    });
    normalNodeIds.add(node.id);

    const sccIndex = sccResult.nodeToComponent.get(node.id);
    const sccComponent = sccIndex !== undefined ? sccResult.components[sccIndex] : undefined;
    const isInScc = sccComponent !== undefined && sccComponent.length >= SCC_MIN_SIZE;
    normalBaseNodes.push(createNormalNode(node, isInScc));
  }

  const drawnEdges: Edge[] = expanded.edges
    .slice()
    .sort(compareEdgesForLayout)
    .map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      animated: edge.flowPerMin > 0.9,
      label: `${edge.flowPerMin.toFixed(2)}/min`,
      style: 'stroke-width:1.6;stroke:#2f4a53;',
      labelStyle: 'font-size:11px;fill:#1f353f;'
    }));

  const filteredSccResult = buildFilteredSccResult(sccResult, normalNodeIds);

  const positionedNormals = layoutNodesWithDagre(
    normalBaseNodes,
    drawnEdges,
    normalLayoutNodes,
    filteredSccResult.components.length > 0 ? filteredSccResult : undefined
  );

  const allNodes = [...positionedNormals, ...lightweightBaseNodes];
  const positionedNodes = positionLightweightNodes(
    allNodes,
    expanded.lightweightNodeIds,
    expanded.lightweightToTarget
  );

  const highlightIndex = buildFlowGraphHighlightIndex({
    nodeIds: positionedNodes.map((node) => node.id),
    edges: drawnEdges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target
    })),
    sccResult,
    lightweightNodeIds: expanded.lightweightNodeIds,
    lightweightToTarget: expanded.lightweightToTarget
  });

  return {
    nodes: positionedNodes,
    edges: drawnEdges,
    highlightIndex
  };
}
