import type { Edge, Node } from '@xyflow/svelte';

export type FlowGraphNodeId = string;
export type FlowGraphEdgeId = string;

/**
 * A node id in the SCC-collapsed graph.
 *
 * - `node:<id>`: a regular node not considered as part of an SCC cluster.
 * - `scc:<index>`: an SCC cluster (treated as a single super node).
 */
export type CollapsedNodeId = `node:${string}` | `scc:${number}`;

export type GraphHighlightDirection = 'upstream' | 'downstream' | 'both';

export type GraphSccTraversalMode =
  /** Do not traverse inside SCCs; treat each SCC as a single super node. */
  | 'collapsed'
  /** Traverse individual nodes inside SCCs (kept for future UX/experiments). */
  | 'expanded';

export interface GraphHighlightRequest {
  startNodeId: FlowGraphNodeId;
  direction: GraphHighlightDirection;
  sccTraversal: GraphSccTraversalMode;
}

export interface FlowGraphEdgeRecord {
  id: FlowGraphEdgeId;
  source: FlowGraphNodeId;
  target: FlowGraphNodeId;
}

/**
 * A purely-structural index for reachability / highlighting.
 *
 * Note: this is intentionally decoupled from XYFlow's `Node`/`Edge` types.
 */
export interface FlowGraphHighlightIndex {
  nodeIds: ReadonlySet<FlowGraphNodeId>;
  edgeIds: ReadonlySet<FlowGraphEdgeId>;

  edges: ReadonlyArray<FlowGraphEdgeRecord>;

  /** Concrete graph adjacency (over rendered node ids). */
  out: ReadonlyMap<FlowGraphNodeId, ReadonlySet<FlowGraphNodeId>>;
  in: ReadonlyMap<FlowGraphNodeId, ReadonlySet<FlowGraphNodeId>>;

  /** Original SCC information over the rendered node set. */
  scc: {
    components: ReadonlyArray<ReadonlyArray<FlowGraphNodeId>>;
    nodeToComponent: ReadonlyMap<FlowGraphNodeId, number>;
  };

  /** Maps a concrete node id to its SCC-collapsed representative. */
  nodeToCollapsed: ReadonlyMap<FlowGraphNodeId, CollapsedNodeId>;
  /** Inverse of `nodeToCollapsed`. */
  collapsedToNodes: ReadonlyMap<CollapsedNodeId, ReadonlyArray<FlowGraphNodeId>>;

  /** Adjacency of the SCC-collapsed graph. */
  collapsedOut: ReadonlyMap<CollapsedNodeId, ReadonlySet<CollapsedNodeId>>;
  collapsedIn: ReadonlyMap<CollapsedNodeId, ReadonlySet<CollapsedNodeId>>;

  /** Synthetic nodes created for external_supply fan-out reduction. */
  lightweightNodeIds: ReadonlySet<FlowGraphNodeId>;
  /** Lightweight node -> downstream target mapping (render-graph edge already uses this as source). */
  lightweightToTarget: ReadonlyMap<FlowGraphNodeId, FlowGraphNodeId>;
}

export interface GraphHighlightSelection {
  /** Concrete node ids (XYFlow `Node.id`). */
  nodeIds: ReadonlySet<FlowGraphNodeId>;
  /** Concrete edge ids (XYFlow `Edge.id`). */
  edgeIds: ReadonlySet<FlowGraphEdgeId>;
}

export interface LayoutNode {
  id: string;
  width: number;
  height: number;
  xOffset: number;
  yOffset: number;
}

export interface SCCResult {
  /** 每个 SCC 包含的节点 ID 列表 */
  components: string[][];
  /** 节点到其 SCC 索引的映射 */
  nodeToComponent: Map<string, number>;
  /** 缩合图的边（DAG） */
  condensedEdges: Set<string>;
}

export interface BuildFlowGraphResult {
  nodes: Node[];
  edges: Edge[];
  highlightIndex: FlowGraphHighlightIndex;
}

export const NODE_WIDTH = 220;
export const NODE_HEIGHT = 44;
export const NODE_X_OFFSET = NODE_WIDTH / 2;
export const NODE_Y_OFFSET = NODE_HEIGHT / 2;

export const LIGHTWEIGHT_NODE_WIDTH = 12;
export const LIGHTWEIGHT_NODE_HEIGHT = 12;
export const LIGHTWEIGHT_NODE_Y_OFFSET = LIGHTWEIGHT_NODE_HEIGHT / 2;

export const SCC_CLUSTER_PADDING = 32;
export const SCC_MIN_SIZE = 2; // SCC 中至少 2 个节点才被视为强连通分量
