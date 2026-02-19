import dagre from '@dagrejs/dagre';
import type { Edge, Node, Position } from '@xyflow/svelte';
import type { LogisticsGraphDto, LogisticsNodeDto } from './types';

const NODE_WIDTH = 220;
const NODE_HEIGHT = 44;
const NODE_X_OFFSET = NODE_WIDTH / 2;
const NODE_Y_OFFSET = NODE_HEIGHT / 2;

const LIGHTWEIGHT_NODE_WIDTH = 12;
const LIGHTWEIGHT_NODE_HEIGHT = 12;
const LIGHTWEIGHT_NODE_X_OFFSET = LIGHTWEIGHT_NODE_WIDTH / 2;
const LIGHTWEIGHT_NODE_Y_OFFSET = LIGHTWEIGHT_NODE_HEIGHT / 2;

const SCC_CLUSTER_PADDING = 32;
const SCC_MIN_SIZE = 2; // SCC 中至少 2 个节点才被视为强连通分量

interface SCCResult {
  /** 每个 SCC 包含的节点 ID 列表 */
  components: string[][];
  /** 节点到其 SCC 索引的映射 */
  nodeToComponent: Map<string, number>;
  /** 缩合图的边（DAG） */
  condensedEdges: Set<string>;
}

const kindColor: Record<string, string> = {
  external_supply: '#2f8f83',
  recipe_group: '#2a678a',
  outpost_sale: '#b16d00',
  thermal_bank_group: '#8b305e'
};

function pickNodeColor(kind: string): string {
  return kindColor[kind] ?? '#3f5165';
}

function compareNodesByLabel(lhs: { label: string; id: string }, rhs: { label: string; id: string }): number {
  return lhs.label.localeCompare(rhs.label) || lhs.id.localeCompare(rhs.id);
}

/**
 * 使用 Tarjan 算法识别强连通分量（SCC）
 * 返回每个 SCC 的节点列表以及节点到 SCC 的映射
 */
function findSCCs(
  nodeIds: string[],
  edges: { source: string; target: string }[]
): SCCResult {
  const adjacency = new Map<string, string[]>();
  for (const id of nodeIds) {
    adjacency.set(id, []);
  }
  for (const edge of edges) {
    const list = adjacency.get(edge.source);
    if (list) {
      list.push(edge.target);
    }
  }

  let index = 0;
  const stack: string[] = [];
  const onStack = new Set<string>();
  const indices = new Map<string, number>();
  const lowlinks = new Map<string, number>();
  const components: string[][] = [];

  function strongConnect(node: string): void {
    indices.set(node, index);
    lowlinks.set(node, index);
    index++;
    stack.push(node);
    onStack.add(node);

    const neighbors = adjacency.get(node) ?? [];
    for (const neighbor of neighbors) {
      if (!indices.has(neighbor)) {
        strongConnect(neighbor);
        const currentLow = lowlinks.get(node) ?? index;
        const neighborLow = lowlinks.get(neighbor) ?? index;
        lowlinks.set(node, Math.min(currentLow, neighborLow));
      } else if (onStack.has(neighbor)) {
        const currentLow = lowlinks.get(node) ?? index;
        const neighborIndex = indices.get(neighbor) ?? index;
        lowlinks.set(node, Math.min(currentLow, neighborIndex));
      }
    }

    const nodeLow = lowlinks.get(node) ?? -1;
    const nodeIndex = indices.get(node) ?? -1;
    if (nodeLow === nodeIndex) {
      const component: string[] = [];
      let w: string;
      do {
        w = stack.pop()!;
        onStack.delete(w);
        component.push(w);
      } while (w !== node);
      components.push(component);
    }
  }

  for (const node of nodeIds) {
    if (!indices.has(node)) {
      strongConnect(node);
    }
  }

  // 创建节点到 SCC 索引的映射
  const nodeToComponent = new Map<string, number>();
  for (let i = 0; i < components.length; i++) {
    for (const node of components[i]) {
      nodeToComponent.set(node, i);
    }
  }

  // 构建缩合图的边（DAG）
  const condensedEdges = new Set<string>();
  for (const edge of edges) {
    const sourceComp = nodeToComponent.get(edge.source);
    const targetComp = nodeToComponent.get(edge.target);
    if (sourceComp !== undefined && targetComp !== undefined && sourceComp !== targetComp) {
      condensedEdges.add(`${sourceComp}->${targetComp}`);
    }
  }

  return { components, nodeToComponent, condensedEdges };
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

interface LayoutNode {
  id: string;
  width: number;
  height: number;
  xOffset: number;
  yOffset: number;
}

interface SCCCluster {
  id: string;
  nodeIds: string[];
  width: number;
  height: number;
  xOffset: number;
  yOffset: number;
  label?: string;
}

/**
 * 计算 SCC 子图的包围盒尺寸
 */
function calculateClusterDimensions(
  nodeIds: string[],
  nodeMap: Map<string, LayoutNode>,
  padding: number
): { width: number; height: number } {
  let totalWidth = 0;
  let maxHeight = 0;
  for (const id of nodeIds) {
    const node = nodeMap.get(id);
    if (node) {
      totalWidth += node.width;
      maxHeight = Math.max(maxHeight, node.height);
    }
  }
  // 使用 dagre 的聚类布局时，cluster 的 padding 会在内部节点周围额外增加
  // 这里我们预留足够的空间给内部布局
  return {
    width: Math.max(totalWidth + padding * 2, NODE_WIDTH + padding * 2),
    height: maxHeight + padding * 3 // 额外空间给标签
  };
}

function layoutNodesWithDagre(
  nodes: Node[],
  edges: Edge[],
  layoutNodes: LayoutNode[],
  sccResult?: SCCResult
): Node[] {
  const graph = new dagre.graphlib.Graph({ compound: true, multigraph: true });
  graph.setDefaultEdgeLabel(() => ({}));
  graph.setGraph({
    rankdir: 'LR',
    ranksep: 140,
    nodesep: 36,
    marginx: 24,
    marginy: 24,
    // 启用聚类布局优化
    clusterPadding: SCC_CLUSTER_PADDING
  });

  const layoutNodeMap = new Map<string, LayoutNode>();
  for (const ln of layoutNodes) {
    layoutNodeMap.set(ln.id, ln);
  }

  // 识别需要作为子图的 SCC（大小 >= SCC_MIN_SIZE）
  const clusters: SCCCluster[] = [];
  const nodeToCluster = new Map<string, string>();

  if (sccResult) {
    for (let i = 0; i < sccResult.components.length; i++) {
      const component = sccResult.components[i];
      if (component.length >= SCC_MIN_SIZE) {
        const clusterId = `__scc_cluster_${i}`;
        const dims = calculateClusterDimensions(component, layoutNodeMap, SCC_CLUSTER_PADDING);
        clusters.push({
          id: clusterId,
          nodeIds: component,
          width: dims.width,
          height: dims.height,
          xOffset: dims.width / 2,
          yOffset: dims.height / 2,
          label: `SCC ${i + 1} (${component.length})`
        });
        for (const nodeId of component) {
          nodeToCluster.set(nodeId, clusterId);
        }
      }
    }
  }

  // 先创建所有节点（包括 cluster 节点）
  for (const cluster of clusters) {
    graph.setNode(cluster.id, {
      width: cluster.width,
      height: cluster.height,
      label: cluster.label
    });
  }

  for (const node of nodes) {
    const ln = layoutNodeMap.get(node.id);
    if (ln) {
      graph.setNode(node.id, {
        width: ln.width,
        height: ln.height
      });
    }
  }

  // 建立父子关系：将节点放入对应的 cluster
  for (const cluster of clusters) {
    for (const nodeId of cluster.nodeIds) {
      graph.setParent(nodeId, cluster.id);
    }
  }

  for (const edge of edges) {
    graph.setEdge(edge.source, edge.target, {}, edge.id);
  }

  dagre.layout(graph);

  return nodes.map((node) => {
    const point = graph.node(node.id) as { x: number; y: number } | undefined;
    const ln = layoutNodeMap.get(node.id);
    if (!point || !ln) {
      return node;
    }

    return {
      ...node,
      position: {
        x: point.x - ln.xOffset,
        y: point.y - ln.yOffset
      }
    };
  });
}

/**
 * 为每个外部供给节点生成轻量级的拆分节点。
 * 如果一个外部供给节点连接了 N 个下游节点，就创建 N 个轻量级节点，
 * 每个轻量级节点连接到一个下游节点。这样可以避免从一个点发散出太多线。
 */
function expandExternalSupplyNodes(
  nodes: LogisticsNodeDto[],
  edges: { id: string; source: string; target: string; itemKey: string; flowPerMin: number }[]
): {
  nodes: LogisticsNodeDto[];
  edges: { id: string; source: string; target: string; itemKey: string; flowPerMin: number }[];
  lightweightNodeIds: Set<string>;
  /** 轻量级节点到下游目标节点的映射 */
  lightweightToTarget: Map<string, string>;
} {
  const nodeMap = new Map<string, LogisticsNodeDto>();
  for (const node of nodes) {
    nodeMap.set(node.id, node);
  }

  // 找出所有外部供给节点
  const externalSupplyIds = new Set<string>();
  for (const node of nodes) {
    if (node.kind === 'external_supply') {
      externalSupplyIds.add(node.id);
    }
  }

  // 为每个外部供给边创建轻量级节点
  const newNodes: LogisticsNodeDto[] = [];
  const newEdges: { id: string; source: string; target: string; itemKey: string; flowPerMin: number }[] = [];
  const lightweightNodeIds = new Set<string>();
  const lightweightToTarget = new Map<string, string>();
  const expandedOriginalIds = new Set<string>();

  for (const edge of edges) {
    if (externalSupplyIds.has(edge.source)) {
      // 这是一个从外部供给节点出发的边
      const originalNode = nodeMap.get(edge.source);
      if (originalNode) {
        // 创建轻量级节点
        const lightweightNodeId = `${edge.source}__lw__${edge.target}`;
        lightweightNodeIds.add(lightweightNodeId);
        lightweightToTarget.set(lightweightNodeId, edge.target);
        expandedOriginalIds.add(edge.source);

        newNodes.push({
          id: lightweightNodeId,
          kind: 'external_supply',
          label: '' // 轻量级节点不显示标签
        });

        newEdges.push({
          ...edge,
          source: lightweightNodeId
        });
      }
    } else {
      // 非外部供给边，保持不变
      newEdges.push(edge);
    }
  }

  // 添加非外部供给节点
  for (const node of nodes) {
    if (node.kind !== 'external_supply') {
      newNodes.push(node);
    } else if (!expandedOriginalIds.has(node.id)) {
      // 外部供给节点但没有下游边（孤立节点），保留原节点
      newNodes.push(node);
    }
  }

  return { nodes: newNodes, edges: newEdges, lightweightNodeIds, lightweightToTarget };
}

/**
 * 在 dagre 布局完成后，根据下游节点的位置定位轻量级节点。
 * 轻量级节点会被放置在下游节点的左侧，垂直居中对齐。
 */
function positionLightweightNodes(
  nodes: Node[],
  lightweightNodeIds: Set<string>,
  lightweightToTarget: Map<string, string>
): Node[] {
  const nodeMap = new Map<string, Node>();
  for (const node of nodes) {
    nodeMap.set(node.id, node);
  }

  const HORIZONTAL_GAP = 56; // 轻量节点与下游节点的水平间距

  return nodes.map((node) => {
    if (!lightweightNodeIds.has(node.id)) {
      return node;
    }

    const targetId = lightweightToTarget.get(node.id);
    const targetNode = targetId ? nodeMap.get(targetId) : undefined;

    if (targetNode?.position) {
      // 将轻量节点放在下游节点的左侧，垂直居中对齐
      return {
        ...node,
        position: {
          x: targetNode.position.x - LIGHTWEIGHT_NODE_WIDTH - HORIZONTAL_GAP,
          y: targetNode.position.y + NODE_Y_OFFSET - LIGHTWEIGHT_NODE_Y_OFFSET
        }
      };
    }

    return node;
  });
}

export function buildFlowGraph(
  graph: LogisticsGraphDto
): { nodes: Node[]; edges: Edge[] } {
  // 首先展开外部供给节点
  const expanded = expandExternalSupplyNodes(graph.nodes, graph.edges);

  const nodeIdSet = new Set<string>();
  for (const edge of expanded.edges) {
    nodeIdSet.add(edge.source);
    nodeIdSet.add(edge.target);
  }

  const nodes = expanded.nodes.filter((node) => nodeIdSet.has(node.id)).sort(compareNodesByLabel);

  // 识别 SCC（用于子图布局）
  const allNodeIds = nodes.map((n) => n.id);
  const sccResult = findSCCs(
    allNodeIds,
    expanded.edges.map((e) => ({ source: e.source, target: e.target }))
  );

  // 分离轻量级节点和普通节点，轻量级节点不参与 dagre 布局
  const normalLayoutNodes: LayoutNode[] = [];
  const lightweightBaseNodes: Node[] = [];
  const normalBaseNodes: Node[] = [];
  const normalNodeIds = new Set<string>();

  for (const node of nodes) {
    const isLightweight = expanded.lightweightNodeIds.has(node.id);
    const color = pickNodeColor(node.kind);

    if (isLightweight) {
      // 轻量级节点：更小、无边框阴影、无文字，只有输出 handle（右边）
      lightweightBaseNodes.push({
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
      });
    } else {
      // 普通节点
      normalLayoutNodes.push({
        id: node.id,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        xOffset: NODE_X_OFFSET,
        yOffset: NODE_Y_OFFSET
      });
      normalNodeIds.add(node.id);

      // 获取 SCC 信息用于样式标注
      const sccIndex = sccResult.nodeToComponent.get(node.id);
      const sccComponent = sccIndex !== undefined ? sccResult.components[sccIndex] : undefined;
      const isInSCC = sccComponent !== undefined && sccComponent.length >= SCC_MIN_SIZE;

      // 输出节点（outpost_sale）只有输入 handle（左边）
      const isOutputNode = node.kind === 'outpost_sale';

      // SCC 节点使用特殊边框样式
      const borderStyle = isInSCC
        ? `border:2px solid ${color};box-shadow:0 0 0 1px ${color}40,0 8px 18px -16px rgba(0,0,0,0.65);`
        : `border:1px solid ${color};box-shadow:0 8px 18px -16px rgba(0,0,0,0.65);`;

      normalBaseNodes.push({
        id: node.id,
        type: isOutputNode ? 'output' : undefined,
        data: {
          label: node.label,
          isInSCC
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
      });
    }
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

  // 过滤出只包含普通节点的 SCC（轻量级节点不参与聚类）
  const filteredSccResult: SCCResult = {
    components: sccResult.components
      .map((comp) => comp.filter((id) => normalNodeIds.has(id)))
      .filter((comp) => comp.length >= SCC_MIN_SIZE),
    nodeToComponent: new Map(),
    condensedEdges: sccResult.condensedEdges
  };

  // 重建 nodeToComponent 映射
  for (let i = 0; i < filteredSccResult.components.length; i++) {
    for (const nodeId of filteredSccResult.components[i]) {
      filteredSccResult.nodeToComponent.set(nodeId, i);
    }
  }

  // 只对普通节点进行 dagre 布局（启用 SCC 子图聚类）
  const positionedNormals = layoutNodesWithDagre(
    normalBaseNodes,
    drawnEdges,
    normalLayoutNodes,
    filteredSccResult.components.length > 0 ? filteredSccResult : undefined
  );

  // 后处理定位轻量级节点（根据下游节点位置）
  const allNodes = [...positionedNormals, ...lightweightBaseNodes];
  const positionedNodes = positionLightweightNodes(
    allNodes,
    expanded.lightweightNodeIds,
    expanded.lightweightToTarget
  );

  return {
    nodes: positionedNodes,
    edges: drawnEdges
  };
}
