import type { Edge, Node } from "@xyflow/svelte";
import type { Viewport } from "@xyflow/system";
import { writable } from "svelte/store";

export type FlowSnapshot = {
  nodes: Node[];
  edges: Edge[];
  viewport: Viewport;
};

export const currentFlowSnapshot = writable<FlowSnapshot | null>(null);
