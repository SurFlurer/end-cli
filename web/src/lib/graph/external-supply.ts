import type { Node } from '@xyflow/svelte';
import type { LogisticsNodeDto } from '../types';
import {
  LIGHTWEIGHT_NODE_HEIGHT,
  LIGHTWEIGHT_NODE_WIDTH,
  LIGHTWEIGHT_NODE_Y_OFFSET,
  NODE_Y_OFFSET
} from './types';

interface ExpandableEdge {
  id: string;
  source: string;
  target: string;
}

/**
 * 为每个外部供给节点生成轻量级的拆分节点。
 * 如果一个外部供给节点连接了 N 个下游节点，就创建 N 个轻量级节点，
 * 每个轻量级节点连接到一个下游节点。这样可以避免从一个点发散出太多线。
 */
export function expandExternalSupplyNodes<TEdge extends ExpandableEdge>(
  nodes: LogisticsNodeDto[],
  edges: ReadonlyArray<TEdge>
): {
  nodes: LogisticsNodeDto[];
  edges: TEdge[];
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
  const newEdges: TEdge[] = [];
  const lightweightNodeIds = new Set<string>();
  const lightweightToTarget = new Map<string, string>();
  const expandedOriginalIds = new Set<string>();

  for (const edge of edges) {
    if (externalSupplyIds.has(edge.source)) {
      const originalNode = nodeMap.get(edge.source);
      if (!originalNode) {
        continue;
      }

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
      } as TEdge);
    } else {
      // 非外部供给边，保持不变
      newEdges.push(edge as TEdge);
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
export function positionLightweightNodes(
  nodes: Node[],
  lightweightNodeIds: ReadonlySet<string>,
  lightweightToTarget: ReadonlyMap<string, string>
): Node[] {
  const nodeMap = new Map<string, Node>();
  for (const node of nodes) {
    nodeMap.set(node.id, node);
  }

  const horizontalGap = 56;

  return nodes.map((node) => {
    if (!lightweightNodeIds.has(node.id)) {
      return node;
    }

    const targetId = lightweightToTarget.get(node.id);
    const targetNode = targetId ? nodeMap.get(targetId) : undefined;

    if (!targetNode?.position) {
      return node;
    }

    return {
      ...node,
      position: {
        x: targetNode.position.x - LIGHTWEIGHT_NODE_WIDTH - horizontalGap,
        y: targetNode.position.y + NODE_Y_OFFSET - LIGHTWEIGHT_NODE_Y_OFFSET
      }
    };
  });
}
