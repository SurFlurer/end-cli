import {
  SCC_MIN_SIZE,
  type CollapsedNodeId,
  type FlowGraphEdgeRecord,
  type FlowGraphHighlightIndex,
  type FlowGraphNodeId,
  type GraphHighlightRequest,
  type GraphHighlightSelection,
  type SCCResult
} from './types';

interface BuildFlowGraphHighlightIndexInput {
  nodeIds: ReadonlyArray<FlowGraphNodeId>;
  edges: ReadonlyArray<FlowGraphEdgeRecord>;
  sccResult: SCCResult;
  lightweightNodeIds: ReadonlySet<FlowGraphNodeId>;
  lightweightToTarget: ReadonlyMap<FlowGraphNodeId, FlowGraphNodeId>;
}

interface FlowStatsByNode<TNodeId extends string> {
  totalOutByNode: Map<TNodeId, number>;
  outFlowByNodePair: Map<TNodeId, Map<TNodeId, number>>;
}

const FLOW_EPSILON = 1e-9;

function ensureSetEntry<K, V>(map: Map<K, Set<V>>, key: K): Set<V> {
  const existing = map.get(key);
  if (existing) {
    return existing;
  }

  const created = new Set<V>();
  map.set(key, created);
  return created;
}

function ensureMapEntry<K, V>(map: Map<K, V>, key: K, create: () => V): V {
  const existing = map.get(key);
  if (existing !== undefined) {
    return existing;
  }

  const created = create();
  map.set(key, created);
  return created;
}

function normalizeRenderedScc(
  renderedNodeIds: ReadonlySet<FlowGraphNodeId>,
  sccResult: SCCResult
): {
  components: FlowGraphNodeId[][];
  nodeToComponent: Map<FlowGraphNodeId, number>;
} {
  const components: FlowGraphNodeId[][] = [];
  const nodeToComponent = new Map<FlowGraphNodeId, number>();

  for (const component of sccResult.components) {
    const filtered = component.filter((nodeId) => renderedNodeIds.has(nodeId));
    if (filtered.length === 0) {
      continue;
    }

    const nextIndex = components.length;
    components.push(filtered);
    for (const nodeId of filtered) {
      nodeToComponent.set(nodeId, nextIndex);
    }
  }

  // 防御：确保每个渲染节点都能映射到一个组件。
  for (const nodeId of renderedNodeIds) {
    if (nodeToComponent.has(nodeId)) {
      continue;
    }

    const nextIndex = components.length;
    components.push([nodeId]);
    nodeToComponent.set(nodeId, nextIndex);
  }

  return { components, nodeToComponent };
}

function collectReachable<T extends string>(
  startId: T,
  adjacency: ReadonlyMap<T, ReadonlySet<T>>
): Set<T> {
  const visited = new Set<T>([startId]);
  const stack: T[] = [startId];

  while (stack.length > 0) {
    const current = stack.pop()!;
    const neighbors = adjacency.get(current);
    if (!neighbors) {
      continue;
    }

    for (const next of neighbors) {
      if (visited.has(next)) {
        continue;
      }

      visited.add(next);
      stack.push(next);
    }
  }

  return visited;
}

function collectEdgeIdsWithinNodes(
  edges: ReadonlyArray<FlowGraphEdgeRecord>,
  selectedNodeIds: ReadonlySet<FlowGraphNodeId>
): Set<string> {
  const selectedEdgeIds = new Set<string>();
  for (const edge of edges) {
    if (selectedNodeIds.has(edge.source) && selectedNodeIds.has(edge.target)) {
      selectedEdgeIds.add(edge.id);
    }
  }
  return selectedEdgeIds;
}

function collectEdgeIdsWithinCollapsedNodes(
  index: FlowGraphHighlightIndex,
  selectedCollapsedNodeIds: ReadonlySet<CollapsedNodeId>
): Set<string> {
  const selectedEdgeIds = new Set<string>();

  for (const edge of index.edges) {
    const sourceCollapsed = index.nodeToCollapsed.get(edge.source);
    const targetCollapsed = index.nodeToCollapsed.get(edge.target);
    if (!sourceCollapsed || !targetCollapsed) {
      continue;
    }

    if (selectedCollapsedNodeIds.has(sourceCollapsed) && selectedCollapsedNodeIds.has(targetCollapsed)) {
      selectedEdgeIds.add(edge.id);
    }
  }

  return selectedEdgeIds;
}

function buildCollapsedFlowStats(index: FlowGraphHighlightIndex): FlowStatsByNode<CollapsedNodeId> {
  const totalOutByNode = new Map<CollapsedNodeId, number>();
  const outFlowByNodePair = new Map<CollapsedNodeId, Map<CollapsedNodeId, number>>();

  for (const edge of index.edges) {
    const sourceCollapsed = index.nodeToCollapsed.get(edge.source);
    const targetCollapsed = index.nodeToCollapsed.get(edge.target);
    if (!sourceCollapsed || !targetCollapsed || sourceCollapsed === targetCollapsed) {
      continue;
    }

    const flowPerMin = Math.max(0, edge.flowPerMin);
    if (flowPerMin <= FLOW_EPSILON) {
      continue;
    }

    totalOutByNode.set(sourceCollapsed, (totalOutByNode.get(sourceCollapsed) ?? 0) + flowPerMin);
    const flowByTarget = ensureMapEntry(outFlowByNodePair, sourceCollapsed, () => new Map<CollapsedNodeId, number>());
    flowByTarget.set(targetCollapsed, (flowByTarget.get(targetCollapsed) ?? 0) + flowPerMin);
  }

  return {
    totalOutByNode,
    outFlowByNodePair
  };
}

function buildConcreteFlowStats(index: FlowGraphHighlightIndex): FlowStatsByNode<FlowGraphNodeId> {
  const totalOutByNode = new Map<FlowGraphNodeId, number>();
  const outFlowByNodePair = new Map<FlowGraphNodeId, Map<FlowGraphNodeId, number>>();

  for (const edge of index.edges) {
    const flowPerMin = Math.max(0, edge.flowPerMin);
    if (flowPerMin <= FLOW_EPSILON) {
      continue;
    }

    totalOutByNode.set(edge.source, (totalOutByNode.get(edge.source) ?? 0) + flowPerMin);
    const flowByTarget = ensureMapEntry(outFlowByNodePair, edge.source, () => new Map<FlowGraphNodeId, number>());
    flowByTarget.set(edge.target, (flowByTarget.get(edge.target) ?? 0) + flowPerMin);
  }

  return {
    totalOutByNode,
    outFlowByNodePair
  };
}

function computeUsageShareOnCollapsedGraph(
  index: FlowGraphHighlightIndex,
  startCollapsedNodeId: CollapsedNodeId,
  upstreamCollapsedNodeIds: ReadonlySet<CollapsedNodeId>
): Map<CollapsedNodeId, number> {
  const flowStats = buildCollapsedFlowStats(index);
  const shareByCollapsedNode = new Map<CollapsedNodeId, number>([[startCollapsedNodeId, 1]]);
  const visiting = new Set<CollapsedNodeId>();

  function solve(nodeId: CollapsedNodeId): number {
    const cached = shareByCollapsedNode.get(nodeId);
    if (cached !== undefined) {
      return cached;
    }

    if (!upstreamCollapsedNodeIds.has(nodeId)) {
      return 0;
    }

    if (visiting.has(nodeId)) {
      // 防御：collapsed 图应是 DAG，这里只在异常输入时兜底。
      return 0;
    }
    visiting.add(nodeId);

    const totalOut = flowStats.totalOutByNode.get(nodeId) ?? 0;
    if (totalOut <= FLOW_EPSILON) {
      shareByCollapsedNode.set(nodeId, 0);
      visiting.delete(nodeId);
      return 0;
    }

    let share = 0;
    const downstreamNodes = index.collapsedOut.get(nodeId);
    if (downstreamNodes) {
      for (const downstreamNodeId of downstreamNodes) {
        if (!upstreamCollapsedNodeIds.has(downstreamNodeId)) {
          continue;
        }

        const flowToDownstream =
          flowStats.outFlowByNodePair.get(nodeId)?.get(downstreamNodeId) ?? 0;
        if (flowToDownstream <= FLOW_EPSILON) {
          continue;
        }

        share += (flowToDownstream / totalOut) * solve(downstreamNodeId);
      }
    }

    const clampedShare = Math.min(1, Math.max(0, share));
    shareByCollapsedNode.set(nodeId, clampedShare);
    visiting.delete(nodeId);
    return clampedShare;
  }

  for (const nodeId of upstreamCollapsedNodeIds) {
    solve(nodeId);
  }

  // 确保起点稳定为 1。
  shareByCollapsedNode.set(startCollapsedNodeId, 1);
  return shareByCollapsedNode;
}

function computeUsageShareOnConcreteGraph(
  index: FlowGraphHighlightIndex,
  startNodeId: FlowGraphNodeId,
  upstreamNodeIds: ReadonlySet<FlowGraphNodeId>
): Map<FlowGraphNodeId, number> {
  const flowStats = buildConcreteFlowStats(index);
  const shareByNode = new Map<FlowGraphNodeId, number>([[startNodeId, 1]]);
  const visiting = new Set<FlowGraphNodeId>();

  function solve(nodeId: FlowGraphNodeId): number {
    const cached = shareByNode.get(nodeId);
    if (cached !== undefined) {
      return cached;
    }

    if (!upstreamNodeIds.has(nodeId)) {
      return 0;
    }

    if (visiting.has(nodeId)) {
      // 防御：expanded 模式可能包含环，这里退化为 0，避免无限递归。
      return 0;
    }
    visiting.add(nodeId);

    const totalOut = flowStats.totalOutByNode.get(nodeId) ?? 0;
    if (totalOut <= FLOW_EPSILON) {
      shareByNode.set(nodeId, 0);
      visiting.delete(nodeId);
      return 0;
    }

    let share = 0;
    const downstreamNodes = index.out.get(nodeId);
    if (downstreamNodes) {
      for (const downstreamNodeId of downstreamNodes) {
        if (!upstreamNodeIds.has(downstreamNodeId)) {
          continue;
        }

        const flowToDownstream =
          flowStats.outFlowByNodePair.get(nodeId)?.get(downstreamNodeId) ?? 0;
        if (flowToDownstream <= FLOW_EPSILON) {
          continue;
        }

        share += (flowToDownstream / totalOut) * solve(downstreamNodeId);
      }
    }

    const clampedShare = Math.min(1, Math.max(0, share));
    shareByNode.set(nodeId, clampedShare);
    visiting.delete(nodeId);
    return clampedShare;
  }

  for (const nodeId of upstreamNodeIds) {
    solve(nodeId);
  }

  shareByNode.set(startNodeId, 1);
  return shareByNode;
}

function buildUpstreamUsedFlowByEdgeIdCollapsed(
  index: FlowGraphHighlightIndex,
  startCollapsedNodeId: CollapsedNodeId,
  upstreamCollapsedNodeIds: ReadonlySet<CollapsedNodeId>,
  upstreamEdgeIds: ReadonlySet<string>
): Map<string, number> {
  const shareByCollapsedNode = computeUsageShareOnCollapsedGraph(
    index,
    startCollapsedNodeId,
    upstreamCollapsedNodeIds
  );

  const upstreamUsedFlowByEdgeId = new Map<string, number>();
  for (const edge of index.edges) {
    if (!upstreamEdgeIds.has(edge.id)) {
      continue;
    }

    const sourceCollapsed = index.nodeToCollapsed.get(edge.source);
    const targetCollapsed = index.nodeToCollapsed.get(edge.target);
    if (!sourceCollapsed || !targetCollapsed) {
      continue;
    }

    const flowPerMin = Math.max(0, edge.flowPerMin);
    if (flowPerMin <= FLOW_EPSILON) {
      upstreamUsedFlowByEdgeId.set(edge.id, 0);
      continue;
    }

    // SCC 内部边按“全部用于该 SCC”处理；SCC 已作为单个大节点参与分摊。
    if (sourceCollapsed === targetCollapsed) {
      upstreamUsedFlowByEdgeId.set(edge.id, flowPerMin);
      continue;
    }

    const usageShare = shareByCollapsedNode.get(targetCollapsed) ?? 0;
    const usedPerMin = Math.min(flowPerMin, Math.max(0, flowPerMin * usageShare));
    upstreamUsedFlowByEdgeId.set(edge.id, usedPerMin);
  }

  return upstreamUsedFlowByEdgeId;
}

function buildUpstreamUsedFlowByEdgeIdExpanded(
  index: FlowGraphHighlightIndex,
  startNodeId: FlowGraphNodeId,
  upstreamNodeIds: ReadonlySet<FlowGraphNodeId>,
  upstreamEdgeIds: ReadonlySet<string>
): Map<string, number> {
  const shareByNode = computeUsageShareOnConcreteGraph(index, startNodeId, upstreamNodeIds);

  const upstreamUsedFlowByEdgeId = new Map<string, number>();
  for (const edge of index.edges) {
    if (!upstreamEdgeIds.has(edge.id)) {
      continue;
    }

    const flowPerMin = Math.max(0, edge.flowPerMin);
    if (flowPerMin <= FLOW_EPSILON) {
      upstreamUsedFlowByEdgeId.set(edge.id, 0);
      continue;
    }

    const usageShare = shareByNode.get(edge.target) ?? 0;
    const usedPerMin = Math.min(flowPerMin, Math.max(0, flowPerMin * usageShare));
    upstreamUsedFlowByEdgeId.set(edge.id, usedPerMin);
  }

  return upstreamUsedFlowByEdgeId;
}

export function buildFlowGraphHighlightIndex(
  input: BuildFlowGraphHighlightIndexInput
): FlowGraphHighlightIndex {
  const nodeIds = new Set<FlowGraphNodeId>(input.nodeIds);

  const out = new Map<FlowGraphNodeId, Set<FlowGraphNodeId>>();
  const incoming = new Map<FlowGraphNodeId, Set<FlowGraphNodeId>>();
  for (const nodeId of nodeIds) {
    out.set(nodeId, new Set<FlowGraphNodeId>());
    incoming.set(nodeId, new Set<FlowGraphNodeId>());
  }

  const edges: FlowGraphEdgeRecord[] = [];
  const edgeIds = new Set<string>();
  for (const edge of input.edges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) {
      continue;
    }

    edges.push({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      flowPerMin: edge.flowPerMin
    });
    edgeIds.add(edge.id);
    ensureSetEntry(out, edge.source).add(edge.target);
    ensureSetEntry(incoming, edge.target).add(edge.source);
  }

  const renderedScc = normalizeRenderedScc(nodeIds, input.sccResult);

  const nodeToCollapsed = new Map<FlowGraphNodeId, CollapsedNodeId>();
  const collapsedToNodes = new Map<CollapsedNodeId, FlowGraphNodeId[]>();

  for (const nodeId of nodeIds) {
    const componentIndex = renderedScc.nodeToComponent.get(nodeId);
    const component = componentIndex === undefined ? undefined : renderedScc.components[componentIndex];

    const collapsedId: CollapsedNodeId =
      component !== undefined && component.length >= SCC_MIN_SIZE
        ? (`scc:${componentIndex}` as CollapsedNodeId)
        : (`node:${nodeId}` as CollapsedNodeId);

    nodeToCollapsed.set(nodeId, collapsedId);
    const bucket = collapsedToNodes.get(collapsedId);
    if (bucket) {
      bucket.push(nodeId);
    } else {
      collapsedToNodes.set(collapsedId, [nodeId]);
    }
  }

  const collapsedOut = new Map<CollapsedNodeId, Set<CollapsedNodeId>>();
  const collapsedIn = new Map<CollapsedNodeId, Set<CollapsedNodeId>>();
  for (const collapsedId of collapsedToNodes.keys()) {
    collapsedOut.set(collapsedId, new Set<CollapsedNodeId>());
    collapsedIn.set(collapsedId, new Set<CollapsedNodeId>());
  }

  for (const edge of edges) {
    const sourceCollapsed = nodeToCollapsed.get(edge.source);
    const targetCollapsed = nodeToCollapsed.get(edge.target);
    if (!sourceCollapsed || !targetCollapsed || sourceCollapsed === targetCollapsed) {
      continue;
    }

    ensureSetEntry(collapsedOut, sourceCollapsed).add(targetCollapsed);
    ensureSetEntry(collapsedIn, targetCollapsed).add(sourceCollapsed);
  }

  return {
    nodeIds,
    edgeIds,
    edges,
    out,
    in: incoming,
    scc: {
      components: renderedScc.components,
      nodeToComponent: renderedScc.nodeToComponent
    },
    nodeToCollapsed,
    collapsedToNodes,
    collapsedOut,
    collapsedIn,
    lightweightNodeIds: new Set(input.lightweightNodeIds),
    lightweightToTarget: new Map(input.lightweightToTarget)
  };
}

function emptySelection(): GraphHighlightSelection {
  return {
    nodeIds: new Set<FlowGraphNodeId>(),
    edgeIds: new Set<string>(),
    upstreamEdgeIds: new Set<string>(),
    downstreamEdgeIds: new Set<string>(),
    upstreamUsedPerMinByEdgeId: new Map<string, number>()
  };
}

function selectExpandedHighlight(
  index: FlowGraphHighlightIndex,
  request: GraphHighlightRequest
): GraphHighlightSelection {
  if (!index.nodeIds.has(request.startNodeId)) {
    return emptySelection();
  }

  const selectedNodeIds = new Set<FlowGraphNodeId>([request.startNodeId]);
  const upstreamNodeIds =
    request.direction === 'upstream' || request.direction === 'both'
      ? collectReachable(request.startNodeId, index.in)
      : new Set<FlowGraphNodeId>();
  const downstreamNodeIds =
    request.direction === 'downstream' || request.direction === 'both'
      ? collectReachable(request.startNodeId, index.out)
      : new Set<FlowGraphNodeId>();

  for (const nodeId of upstreamNodeIds) {
    selectedNodeIds.add(nodeId);
  }
  for (const nodeId of downstreamNodeIds) {
    selectedNodeIds.add(nodeId);
  }

  const selectedEdgeIds = collectEdgeIdsWithinNodes(index.edges, selectedNodeIds);
  const upstreamEdgeIds = collectEdgeIdsWithinNodes(index.edges, upstreamNodeIds);
  const downstreamEdgeIds = collectEdgeIdsWithinNodes(index.edges, downstreamNodeIds);

  return {
    nodeIds: selectedNodeIds,
    edgeIds: selectedEdgeIds,
    upstreamEdgeIds,
    downstreamEdgeIds,
    upstreamUsedPerMinByEdgeId: buildUpstreamUsedFlowByEdgeIdExpanded(
      index,
      request.startNodeId,
      upstreamNodeIds,
      upstreamEdgeIds
    )
  };
}

function selectCollapsedHighlight(
  index: FlowGraphHighlightIndex,
  request: GraphHighlightRequest
): GraphHighlightSelection {
  const startCollapsed = index.nodeToCollapsed.get(request.startNodeId);
  if (!startCollapsed) {
    return emptySelection();
  }

  const selectedCollapsed = new Set<CollapsedNodeId>([startCollapsed]);
  const upstreamCollapsedNodeIds =
    request.direction === 'upstream' || request.direction === 'both'
      ? collectReachable(startCollapsed, index.collapsedIn)
      : new Set<CollapsedNodeId>();
  const downstreamCollapsedNodeIds =
    request.direction === 'downstream' || request.direction === 'both'
      ? collectReachable(startCollapsed, index.collapsedOut)
      : new Set<CollapsedNodeId>();

  for (const collapsedId of upstreamCollapsedNodeIds) {
    selectedCollapsed.add(collapsedId);
  }
  for (const collapsedId of downstreamCollapsedNodeIds) {
    selectedCollapsed.add(collapsedId);
  }

  const selectedNodeIds = new Set<FlowGraphNodeId>();
  for (const collapsedId of selectedCollapsed) {
    const concreteNodes = index.collapsedToNodes.get(collapsedId);
    if (!concreteNodes) {
      continue;
    }

    for (const nodeId of concreteNodes) {
      selectedNodeIds.add(nodeId);
    }
  }

  const selectedEdgeIds = new Set<string>();
  for (const edge of index.edges) {
    const sourceCollapsed = index.nodeToCollapsed.get(edge.source);
    const targetCollapsed = index.nodeToCollapsed.get(edge.target);
    if (!sourceCollapsed || !targetCollapsed) {
      continue;
    }

    if (selectedCollapsed.has(sourceCollapsed) && selectedCollapsed.has(targetCollapsed)) {
      selectedEdgeIds.add(edge.id);
    }
  }

  const upstreamEdgeIds = collectEdgeIdsWithinCollapsedNodes(index, upstreamCollapsedNodeIds);
  const downstreamEdgeIds = collectEdgeIdsWithinCollapsedNodes(index, downstreamCollapsedNodeIds);

  return {
    nodeIds: selectedNodeIds,
    edgeIds: selectedEdgeIds,
    upstreamEdgeIds,
    downstreamEdgeIds,
    upstreamUsedPerMinByEdgeId: buildUpstreamUsedFlowByEdgeIdCollapsed(
      index,
      startCollapsed,
      upstreamCollapsedNodeIds,
      upstreamEdgeIds
    )
  };
}

export function selectGraphHighlight(
  index: FlowGraphHighlightIndex,
  request: GraphHighlightRequest
): GraphHighlightSelection {
  if (request.sccTraversal === 'expanded') {
    return selectExpandedHighlight(index, request);
  }

  return selectCollapsedHighlight(index, request);
}
