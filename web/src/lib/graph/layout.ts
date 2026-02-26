import dagre from '@dagrejs/dagre';
import type { Edge, Node } from '@xyflow/svelte';
import {
  NODE_WIDTH,
  SCC_CLUSTER_PADDING,
  SCC_MIN_SIZE,
  type LayoutNode,
  type SCCResult
} from './types';

interface SCCCluster {
  id: string;
  nodeIds: string[];
  width: number;
  height: number;
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

  // 使用 dagre 的聚类布局时，cluster 的 padding 会在内部节点周围额外增加。
  // 这里预留空间给内部布局和标签。
  return {
    width: Math.max(totalWidth + padding * 2, NODE_WIDTH + padding * 2),
    height: maxHeight + padding * 3
  };
}

export function layoutNodesWithDagre(
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
    clusterPadding: SCC_CLUSTER_PADDING
  });

  const layoutNodeMap = new Map<string, LayoutNode>();
  for (const layoutNode of layoutNodes) {
    layoutNodeMap.set(layoutNode.id, layoutNode);
  }

  // 识别需要作为子图的 SCC（大小 >= SCC_MIN_SIZE）
  const clusters: SCCCluster[] = [];
  if (sccResult) {
    for (let i = 0; i < sccResult.components.length; i++) {
      const component = sccResult.components[i];
      if (component.length < SCC_MIN_SIZE) {
        continue;
      }

      const clusterId = `__scc_cluster_${i}`;
      const dims = calculateClusterDimensions(component, layoutNodeMap, SCC_CLUSTER_PADDING);
      clusters.push({
        id: clusterId,
        nodeIds: component,
        width: dims.width,
        height: dims.height,
        label: `SCC ${i + 1} (${component.length})`
      });
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
    const layoutNode = layoutNodeMap.get(node.id);
    if (!layoutNode) {
      continue;
    }

    graph.setNode(node.id, {
      width: layoutNode.width,
      height: layoutNode.height
    });
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
    const layoutNode = layoutNodeMap.get(node.id);
    if (!point || !layoutNode) {
      return node;
    }

    return {
      ...node,
      position: {
        x: point.x - layoutNode.xOffset,
        y: point.y - layoutNode.yOffset
      }
    };
  });
}
