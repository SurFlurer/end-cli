import dagre from '@dagrejs/dagre';
import type { Edge, Node, Position } from '@xyflow/svelte';
import type { LogisticsGraphDto } from './types';

type FilterKey = string | 'all';

const NODE_WIDTH = 220;
const NODE_HEIGHT = 44;
const NODE_X_OFFSET = NODE_WIDTH / 2;
const NODE_Y_OFFSET = NODE_HEIGHT / 2;

const kindColor: Record<string, string> = {
  external_supply: '#2f8f83',
  recipe_machine: '#2a678a',
  outpost_sale: '#b16d00',
  thermal_bank_fuel: '#8b305e'
};

function pickNodeColor(kind: string): string {
  return kindColor[kind] ?? '#3f5165';
}

function compareNodesByLabel(lhs: { label: string; id: string }, rhs: { label: string; id: string }): number {
  return lhs.label.localeCompare(rhs.label) || lhs.id.localeCompare(rhs.id);
}

function compareEdgesForLayout(
  lhs: { source: string; target: string; itemKey: string; id: string },
  rhs: { source: string; target: string; itemKey: string; id: string }
): number {
  return (
    lhs.source.localeCompare(rhs.source) ||
    lhs.target.localeCompare(rhs.target) ||
    lhs.itemKey.localeCompare(rhs.itemKey) ||
    lhs.id.localeCompare(rhs.id)
  );
}

function layoutNodesWithDagre(nodes: Node[], edges: Edge[]): Node[] {
  const graph = new dagre.graphlib.Graph({ multigraph: true });
  graph.setDefaultEdgeLabel(() => ({}));
  graph.setGraph({
    rankdir: 'LR',
    ranksep: 140,
    nodesep: 36,
    marginx: 24,
    marginy: 24
  });

  for (const node of nodes) {
    graph.setNode(node.id, {
      width: NODE_WIDTH,
      height: NODE_HEIGHT
    });
  }

  for (const edge of edges) {
    graph.setEdge(edge.source, edge.target, {}, edge.id);
  }

  dagre.layout(graph);

  return nodes.map((node) => {
    const point = graph.node(node.id) as { x: number; y: number } | undefined;
    if (!point) {
      return node;
    }

    return {
      ...node,
      position: {
        x: point.x - NODE_X_OFFSET,
        y: point.y - NODE_Y_OFFSET
      }
    };
  });
}

export function buildFlowGraph(
  graph: LogisticsGraphDto,
  filterKey: FilterKey
): { nodes: Node[]; edges: Edge[] } {
  const filteredEdges =
    filterKey === 'all'
      ? graph.edges
      : graph.edges.filter((edge) => edge.itemKey === filterKey);

  const nodeIdSet = new Set<string>();
  for (const edge of filteredEdges) {
    nodeIdSet.add(edge.source);
    nodeIdSet.add(edge.target);
  }

  const nodes = graph.nodes.filter((node) => nodeIdSet.has(node.id)).sort(compareNodesByLabel);
  const baseNodes: Node[] = nodes.map((node) => {
    const color = pickNodeColor(node.kind);
    return {
      id: node.id,
      data: {
        label: node.label
      },
      position: { x: 0, y: 0 },
      sourcePosition: 'right' as Position,
      targetPosition: 'left' as Position,
      style:
        `border:1px solid ${color};` +
        'border-radius:12px;' +
        'background:#fff;' +
        'color:#16313d;' +
        'box-shadow:0 8px 18px -16px rgba(0,0,0,0.65);' +
        'min-width:220px;' +
        'font-size:12px;'
    };
  });

  const drawnEdges: Edge[] = filteredEdges
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

  const positionedNodes = layoutNodesWithDagre(baseNodes, drawnEdges);

  return {
    nodes: positionedNodes,
    edges: drawnEdges
  };
}
