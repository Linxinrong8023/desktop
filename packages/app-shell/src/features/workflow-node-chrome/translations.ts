// Pure translation data: safe to compose without importing feature implementation.
export const workflowNodeTranslations = {
  "zh-CN": {
    "workflowNode.unused": "未参与运行",
    "workflowNode.unusedHint":
      "此节点或所属容器未从作用域入口连通，不会参与运行。",
    "workflowNode.unusedCount": "有 {{count}} 个节点未参与运行",
    "workflowNode.agentExecutionMode.interactive": "人工交互节点",
    "workflowNode.agentExecutionMode.automatic": "自动执行节点",
  },
  "en-US": {
    "workflowNode.unused": "Excluded from execution",
    "workflowNode.unusedHint":
      "This node or its container is not reachable from its scope entry and will not run.",
    "workflowNode.unusedCount": "{{count}} nodes excluded from execution",
    "workflowNode.agentExecutionMode.interactive": "Interactive Agent node",
    "workflowNode.agentExecutionMode.automatic": "Automatic Agent node",
  },
} as const;
