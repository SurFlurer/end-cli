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

function ensureSetEntry<K, V>(map: Map<K, Set<V>>, key: K): Set<V> {
  const existing = map.get(key);
  if (existing) {
    return existing;
  }

  const created = new Set<V>();
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

    edges.push({ id: edge.id, source: edge.source, target: edge.target });
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
    edgeIds: new Set<string>()
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

  if (request.direction === 'upstream' || request.direction === 'both') {
    const upstream = collectReachable(request.startNodeId, index.in);
    for (const nodeId of upstream) {
      selectedNodeIds.add(nodeId);
    }
  }

  if (request.direction === 'downstream' || request.direction === 'both') {
    const downstream = collectReachable(request.startNodeId, index.out);
    for (const nodeId of downstream) {
      selectedNodeIds.add(nodeId);
    }
  }

  return {
    nodeIds: selectedNodeIds,
    edgeIds: collectEdgeIdsWithinNodes(index.edges, selectedNodeIds)
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

  if (request.direction === 'upstream' || request.direction === 'both') {
    const upstream = collectReachable(startCollapsed, index.collapsedIn);
    for (const collapsedId of upstream) {
      selectedCollapsed.add(collapsedId);
    }
  }

  if (request.direction === 'downstream' || request.direction === 'both') {
    const downstream = collectReachable(startCollapsed, index.collapsedOut);
    for (const collapsedId of downstream) {
      selectedCollapsed.add(collapsedId);
    }
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

  return {
    nodeIds: selectedNodeIds,
    edgeIds: selectedEdgeIds
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
