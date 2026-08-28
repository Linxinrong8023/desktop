import type { WorkflowAgentConfig, WorkflowNodeData } from "./node-data";

/**
 * Fills MCP bindings omitted from older drafts so the settings canvas can open
 * without crashing on `agentConfig.mcps`.
 */
export function normalizeWorkflowAgentConfig(
  config: WorkflowAgentConfig,
): WorkflowAgentConfig {
  const skills = Array.isArray(config.skills) ? config.skills : [];
  const mcps = Array.isArray(config.mcps) ? config.mcps : [];
  return {
    ...config,
    skills,
    mcps,
    // Missing `interactive` defaults to false so existing graphs stay fully automatic.
    interactive: config.interactive ?? false,
    // Missing `outputPolicy` defaults to `none` so nodes withhold their output unless opted in.
    outputPolicy: config.outputPolicy ?? "none",
  };
}

/** Normalizes every Agent node in a definition graph envelope. */
export function normalizeWorkflowNodeAgentConfigs<
  T extends {
    data: WorkflowNodeData;
  },
>(nodes: T[]): T[] {
  return nodes.map((node) => {
    if (node.data.kind !== "agent" || node.data.agentConfig === undefined) {
      return node;
    }
    return {
      ...node,
      data: {
        ...node.data,
        agentConfig: normalizeWorkflowAgentConfig(node.data.agentConfig),
      },
    };
  });
}
