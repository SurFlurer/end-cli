import type { SCCResult } from './types';

/**
 * 使用 Tarjan 算法识别强连通分量（SCC）
 * 返回每个 SCC 的节点列表以及节点到 SCC 的映射
 */
export function findSCCs(
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
      let current: string;
      do {
        current = stack.pop()!;
        onStack.delete(current);
        component.push(current);
      } while (current !== node);
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
