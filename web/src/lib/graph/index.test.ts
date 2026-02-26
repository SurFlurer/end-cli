import { describe, expect, it } from 'vitest';
import { buildFlowGraph, selectGraphHighlight } from './index';
import type { LogisticsGraphDto } from '../types';

const graphFixture: LogisticsGraphDto = {
  items: [
    { itemKey: 'A', itemName: 'Item A', edgeCount: 1, nodeCount: 2, totalFlowPerMin: 3.2 },
    { itemKey: 'B', itemName: 'Item B', edgeCount: 1, nodeCount: 2, totalFlowPerMin: 1.1 }
  ],
  nodes: [
    { id: 'n1', kind: 'external_supply', label: 'Supply A' },
    { id: 'n2', kind: 'outpost_sale', label: 'Sale A' },
    { id: 'n3', kind: 'external_supply', label: 'Supply B' },
    { id: 'n4', kind: 'outpost_sale', label: 'Sale B' }
  ],
  edges: [
    {
      id: 'e1',
      itemKey: 'A',
      itemName: 'Item A',
      source: 'n1',
      target: 'n2',
      flowPerMin: 3.2
    },
    {
      id: 'e2',
      itemKey: 'B',
      itemName: 'Item B',
      source: 'n3',
      target: 'n4',
      flowPerMin: 1.1
    }
  ]
};

// 一个外部供给节点连接多个下游节点的测试用例
const multiSupplyFixture: LogisticsGraphDto = {
  items: [
    { itemKey: 'Ore', itemName: 'Iron Ore', edgeCount: 2, nodeCount: 3, totalFlowPerMin: 10 },
    { itemKey: 'Ingot', itemName: 'Iron Ingot', edgeCount: 1, nodeCount: 2, totalFlowPerMin: 5 }
  ],
  nodes: [
    { id: 'supply1', kind: 'external_supply', label: 'Iron Ore Supply' },
    { id: 'recipe1', kind: 'recipe_machine', label: 'Smelter 1' },
    { id: 'recipe2', kind: 'recipe_machine', label: 'Smelter 2' },
    { id: 'sale1', kind: 'outpost_sale', label: 'Sale Point' }
  ],
  edges: [
    {
      id: 'e1',
      itemKey: 'Ore',
      itemName: 'Iron Ore',
      source: 'supply1',
      target: 'recipe1',
      flowPerMin: 6
    },
    {
      id: 'e2',
      itemKey: 'Ore',
      itemName: 'Iron Ore',
      source: 'supply1',
      target: 'recipe2',
      flowPerMin: 4
    },
    {
      id: 'e3',
      itemKey: 'Ingot',
      itemName: 'Iron Ingot',
      source: 'recipe1',
      target: 'sale1',
      flowPerMin: 5
    }
  ]
};

// 带有 SCC 的测试用例（循环依赖）
const sccFixture: LogisticsGraphDto = {
  items: [
    { itemKey: 'A', itemName: 'Item A', edgeCount: 3, nodeCount: 3, totalFlowPerMin: 10 },
    { itemKey: 'B', itemName: 'Item B', edgeCount: 2, nodeCount: 2, totalFlowPerMin: 5 }
  ],
  nodes: [
    { id: 'n1', kind: 'external_supply', label: 'Supply A' },
    { id: 'n2', kind: 'recipe_machine', label: 'Machine 1' },
    { id: 'n3', kind: 'recipe_machine', label: 'Machine 2' },
    { id: 'n4', kind: 'recipe_machine', label: 'Machine 3' },
    { id: 'n5', kind: 'outpost_sale', label: 'Sale' }
  ],
  edges: [
    // 外部供给 -> 机器1
    { id: 'e1', itemKey: 'A', itemName: 'Item A', source: 'n1', target: 'n2', flowPerMin: 5 },
    // SCC: 机器1 -> 机器2 -> 机器3 -> 机器1 (循环依赖)
    { id: 'e2', itemKey: 'B', itemName: 'Item B', source: 'n2', target: 'n3', flowPerMin: 3 },
    { id: 'e3', itemKey: 'A', itemName: 'Item A', source: 'n3', target: 'n4', flowPerMin: 3 },
    { id: 'e4', itemKey: 'B', itemName: 'Item B', source: 'n4', target: 'n2', flowPerMin: 2 },
    // 机器3 -> 销售
    { id: 'e5', itemKey: 'A', itemName: 'Item A', source: 'n4', target: 'n5', flowPerMin: 2 }
  ]
};

// 线性链（无 SCC）
const linearChainFixture: LogisticsGraphDto = {
  items: [{ itemKey: 'A', itemName: 'Item A', edgeCount: 2, nodeCount: 3, totalFlowPerMin: 5 }],
  nodes: [
    { id: 's1', kind: 'external_supply', label: 'Supply' },
    { id: 'm1', kind: 'recipe_machine', label: 'Machine 1' },
    { id: 'm2', kind: 'recipe_machine', label: 'Machine 2' }
  ],
  edges: [
    { id: 'e1', itemKey: 'A', itemName: 'Item A', source: 's1', target: 'm1', flowPerMin: 5 },
    { id: 'e2', itemKey: 'A', itemName: 'Item A', source: 'm1', target: 'm2', flowPerMin: 5 }
  ]
};

const externalConsumptionFixture: LogisticsGraphDto = {
  items: [{ itemKey: 'Ore', itemName: 'Ore', edgeCount: 1, nodeCount: 2, totalFlowPerMin: 2 }],
  nodes: [
    { id: 's1', kind: 'external_supply', label: 'Supply' },
    { id: 'c1', kind: 'external_consumption', label: 'External Consumption' }
  ],
  edges: [
    { id: 'e1', itemKey: 'Ore', itemName: 'Ore', source: 's1', target: 'c1', flowPerMin: 2 }
  ]
};

const upstreamSplitFixture: LogisticsGraphDto = {
  items: [{ itemKey: 'Plate', itemName: 'Plate', edgeCount: 3, nodeCount: 4, totalFlowPerMin: 60 }],
  nodes: [
    { id: 'a', kind: 'external_supply', label: 'A Supply' },
    { id: 'b', kind: 'recipe_machine', label: 'B Machine' },
    { id: 'c', kind: 'outpost_sale', label: 'C Sink' },
    { id: 'd', kind: 'outpost_sale', label: 'D Sink' }
  ],
  edges: [
    { id: 'e_ab', itemKey: 'Plate', itemName: 'Plate', source: 'a', target: 'b', flowPerMin: 20 },
    { id: 'e_bc', itemKey: 'Plate', itemName: 'Plate', source: 'b', target: 'c', flowPerMin: 20 },
    { id: 'e_bd', itemKey: 'Plate', itemName: 'Plate', source: 'b', target: 'd', flowPerMin: 20 }
  ]
};

function sortedValues(values: ReadonlySet<string>): string[] {
  return [...values].sort((lhs, rhs) => lhs.localeCompare(rhs));
}

describe('buildFlowGraph', () => {
  it('returns all nodes and edges', () => {
    const flow = buildFlowGraph(graphFixture);

    expect(flow.edges).toHaveLength(2);
    // 2个轻量级外部供给节点 + 2个普通节点 = 4个节点
    expect(flow.nodes).toHaveLength(4);
    expect(flow.edges[0]?.label).toMatch('/min');
  });

  it('lays out flow direction from left to right', () => {
    const flow = buildFlowGraph(graphFixture);
    // 轻量级节点应该在最左边
    const sourceNode = flow.nodes.find((node) => node.id.includes('n1'));
    const targetNode = flow.nodes.find((node) => node.id === 'n2');

    if (!sourceNode || !targetNode) {
      throw new Error('expected graph to include both chain endpoints');
    }

    expect(sourceNode.position.x).toBeLessThan(targetNode.position.x);
  });

  it('expands external supply node into lightweight nodes for each downstream', () => {
    const flow = buildFlowGraph(multiSupplyFixture);

    // 应该有两个轻量级节点（对应 supply1 的两个下游）
    const lightweightNodes = flow.nodes.filter((node) =>
      node.id.includes('supply1__lw__')
    );
    expect(lightweightNodes).toHaveLength(2);

    // 轻量级节点应该是圆形小点，type 为 'input'（只有输出 handle）
    for (const node of lightweightNodes) {
      expect(node.style).toContain('border-radius:50%');
      expect(node.style).toContain('width:12px');
      expect(node.style).toContain('height:12px');
      expect(node.data?.label).toBe('');
      expect(node.type).toBe('input'); // 只有输出 handle
    }

    // 非外部供给节点应该保持原样
    const recipeNodes = flow.nodes.filter((node) =>
      node.id === 'recipe1' || node.id === 'recipe2' || node.id === 'sale1'
    );
    expect(recipeNodes).toHaveLength(3);

    // 边的 source 应该指向轻量级节点
    const oreEdges = flow.edges.filter((edge) => edge.id === 'e1' || edge.id === 'e2');
    for (const edge of oreEdges) {
      expect(edge.source).toMatch(/^supply1__lw__/);
    }
  });

  it('sets outpost_sale nodes as type output (only input handle)', () => {
    const flow = buildFlowGraph(multiSupplyFixture);

    // outpost_sale 节点应该是 'output' 类型（只有输入 handle）
    const saleNode = flow.nodes.find((node) => node.id === 'sale1');
    expect(saleNode).toBeDefined();
    expect(saleNode?.type).toBe('output');
  });

  it('sets external_consumption nodes as type output (only input handle)', () => {
    const flow = buildFlowGraph(externalConsumptionFixture);

    const consumptionNode = flow.nodes.find((node) => node.id === 'c1');
    expect(consumptionNode).toBeDefined();
    expect(consumptionNode?.type).toBe('output');
  });

  it('keeps recipe_machine nodes with both handles', () => {
    const flow = buildFlowGraph(multiSupplyFixture);

    // recipe_machine 节点应该有默认类型（两边都有 handle）
    const recipeNode = flow.nodes.find((node) => node.id === 'recipe1');
    expect(recipeNode).toBeDefined();
    expect(recipeNode?.type).toBeUndefined();
  });

  it('preserves non-external-supply edges', () => {
    const flow = buildFlowGraph(multiSupplyFixture);

    // recipe1 -> sale1 的边应该保持不变
    const ingotEdge = flow.edges.find((edge) => edge.id === 'e3');
    expect(ingotEdge).toBeDefined();
    expect(ingotEdge?.source).toBe('recipe1');
    expect(ingotEdge?.target).toBe('sale1');
  });

  it('lays out SCC internals from top to bottom', () => {
    const flow = buildFlowGraph(sccFixture);
    const sccNodes = flow.nodes.filter((node) =>
      node.id === 'n2' || node.id === 'n3' || node.id === 'n4'
    );

    const xValues = sccNodes.map((node) => node.position.x);
    const yValues = sccNodes.map((node) => node.position.y);
    const horizontalSpan = Math.max(...xValues) - Math.min(...xValues);
    const verticalSpan = Math.max(...yValues) - Math.min(...yValues);

    expect(verticalSpan).toBeGreaterThan(horizontalSpan);
  });

  it('handles linear chains without SCC styling', () => {
    const flow = buildFlowGraph(linearChainFixture);

    // 线性链不应该有 SCC
    for (const node of flow.nodes) {
      if (!node.id.includes('__lw__')) {
        expect(node.style).toContain('border:1px solid');
        expect(node.data?.isInSCC).toBeFalsy();
      }
    }
  });

  it('maintains DAG structure with SCCs collapsed', () => {
    const flow = buildFlowGraph(sccFixture);

    // 所有节点都应该有位置
    for (const node of flow.nodes) {
      expect(node.position).toBeDefined();
      expect(node.position.x).toBeGreaterThanOrEqual(0);
      expect(node.position.y).toBeGreaterThanOrEqual(0);
    }

    // 边的源和目标都应该存在
    const nodeIds = new Set(flow.nodes.map((n) => n.id));
    for (const edge of flow.edges) {
      expect(nodeIds.has(edge.source)).toBe(true);
      expect(nodeIds.has(edge.target)).toBe(true);
    }
  });
});

describe('selectGraphHighlight', () => {
  it('highlights all upstream and downstream nodes/edges in a linear chain', () => {
    const flow = buildFlowGraph(linearChainFixture);

    const selection = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'm1',
      direction: 'both',
      sccTraversal: 'collapsed'
    });

    expect(sortedValues(selection.nodeIds)).toEqual(['m1', 'm2', 's1__lw__m1']);
    expect(sortedValues(selection.edgeIds)).toEqual(['e1', 'e2']);
  });

  it('returns empty selection when the start node is missing', () => {
    const flow = buildFlowGraph(graphFixture);

    const selection = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'missing-node',
      direction: 'both',
      sccTraversal: 'collapsed'
    });

    expect(selection.nodeIds.size).toBe(0);
    expect(selection.edgeIds.size).toBe(0);
  });

  it('treats SCC as a single super node for collapsed traversal', () => {
    const flow = buildFlowGraph(sccFixture);

    const selection = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'n1__lw__n2',
      direction: 'downstream',
      sccTraversal: 'collapsed'
    });

    expect(sortedValues(selection.nodeIds)).toEqual(['n1__lw__n2', 'n2', 'n3', 'n4', 'n5']);
    expect(sortedValues(selection.edgeIds)).toEqual(['e1', 'e2', 'e3', 'e4', 'e5']);
  });

  it('supports direction-specific traversal', () => {
    const flow = buildFlowGraph(linearChainFixture);

    const upstreamOnly = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'm1',
      direction: 'upstream',
      sccTraversal: 'collapsed'
    });
    const downstreamOnly = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'm1',
      direction: 'downstream',
      sccTraversal: 'collapsed'
    });

    expect(sortedValues(upstreamOnly.nodeIds)).toEqual(['m1', 's1__lw__m1']);
    expect(sortedValues(upstreamOnly.edgeIds)).toEqual(['e1']);

    expect(sortedValues(downstreamOnly.nodeIds)).toEqual(['m1', 'm2']);
    expect(sortedValues(downstreamOnly.edgeIds)).toEqual(['e2']);
  });

  it('classifies upstream and downstream edge sets separately', () => {
    const flow = buildFlowGraph(upstreamSplitFixture);

    const selection = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'b',
      direction: 'both',
      sccTraversal: 'collapsed'
    });

    expect(sortedValues(selection.upstreamEdgeIds)).toEqual(['e_ab']);
    expect(sortedValues(selection.downstreamEdgeIds)).toEqual(['e_bc', 'e_bd']);
  });

  it('computes upstream used flow by proportional downstream usage', () => {
    const flow = buildFlowGraph(upstreamSplitFixture);

    const selection = selectGraphHighlight(flow.highlightIndex, {
      startNodeId: 'c',
      direction: 'both',
      sccTraversal: 'collapsed'
    });

    expect(sortedValues(selection.upstreamEdgeIds)).toEqual(['e_ab', 'e_bc']);
    expect(sortedValues(selection.downstreamEdgeIds)).toEqual([]);

    expect(selection.upstreamUsedPerMinByEdgeId.get('e_bc')).toBeCloseTo(20, 6);
    expect(selection.upstreamUsedPerMinByEdgeId.get('e_ab')).toBeCloseTo(10, 6);
  });
});
