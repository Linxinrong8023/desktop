import { type CSSProperties, type ReactNode, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { IconPlus } from "@tabler/icons-react";
import type { Edge, Node } from "@xyflow/react";
import {
  supportsWorkflowNodeScope,
  type WorkflowCapabilities,
  type WorkflowNodeData,
  type WorkflowNodeKind,
} from "@ora/workflow-mock";
import {
  Button,
  cn,
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@ora/ui";
import type { IterationInsertion } from "../workflow-iteration-graph";
import { getNodeMetadata } from "../workflow-node-metadata";
import {
  useWorkflowIterationActions,
  type WorkflowIterationActions,
  WorkflowIterationActionsContext,
} from "./iteration-actions-context";

/** Provides graph-aware iteration authoring actions to custom nodes and edges. */
export function WorkflowIterationActionsProvider({
  capabilities,
  nodes,
  edges,
  readOnly,
  onInsert,
  onToggleCollapsed,
  children,
}: {
  capabilities: WorkflowCapabilities;
  nodes: Node<WorkflowNodeData, "workflow">[];
  edges: Edge[];
  readOnly: boolean;
  onInsert: (kind: WorkflowNodeKind, insertion: IterationInsertion) => void;
  onToggleCollapsed: (iterationId: string) => void;
  children: ReactNode;
}) {
  const value = useMemo<WorkflowIterationActions>(() => {
    const nodeById = new Map(nodes.map((node) => [node.id, node]));
    const iterationIds = new Set(
      nodes
        .filter((node) => node.data.kind === "iteration")
        .map((node) => node.id),
    );
    const ownerOf = (nodeId: string): string | null => {
      const parentId = nodeById.get(nodeId)?.parentId;
      return parentId !== undefined && iterationIds.has(parentId)
        ? parentId
        : null;
    };
    return {
      nodeTypes: capabilities.nodeTypes.filter((nodeType) =>
        supportsWorkflowNodeScope(nodeType, "iteration"),
      ),
      readOnly,
      insertionForEdge: (edge) => {
        // The fixed start already owns the entry add affordance. Repeating it at the
        // entry edge midpoint makes one logical insertion seam look like two actions.
        if (edge.sourceHandle === "iteration-entry") {
          return null;
        }
        const sourceOwner = ownerOf(edge.source);
        const targetOwner = ownerOf(edge.target);
        const iterationId =
          sourceOwner !== null && sourceOwner === targetOwner
            ? sourceOwner
            : targetOwner !== null && edge.source === targetOwner
              ? targetOwner
              : null;
        return iterationId === null
          ? null
          : { type: "edge", iterationId, edgeId: edge.id };
      },
      outputInsertion: (nodeId, sourceHandle) => {
        const iterationId = ownerOf(nodeId);
        if (iterationId === null) {
          return null;
        }
        const occupied = edges.some(
          (edge) =>
            edge.source === nodeId &&
            (sourceHandle == null
              ? edge.sourceHandle == null
              : edge.sourceHandle === sourceHandle),
        );
        return occupied
          ? null
          : { type: "output", iterationId, sourceId: nodeId, sourceHandle };
      },
      insert: onInsert,
      toggleCollapsed: onToggleCollapsed,
    };
  }, [
    capabilities.nodeTypes,
    edges,
    nodes,
    onInsert,
    onToggleCollapsed,
    readOnly,
  ]);
  return (
    <WorkflowIterationActionsContext.Provider value={value}>
      {children}
    </WorkflowIterationActionsContext.Provider>
  );
}

/** Renders the capability-filtered node picker shared by entry, edge, and output seams.
 * The trigger mirrors Dify's add-block affordance: a solid blue circle with a white
 * plus. Port-anchored seams keep it decorative (`pointer-events-none` via className)
 * and forward port clicks through the controlled `open` state instead, so connection
 * drags from the underlying handle are never intercepted. */
export function IterationInsertMenu({
  insertion,
  label,
  className,
  side = "right",
  style,
  open,
  onOpenChange,
}: {
  insertion: IterationInsertion;
  label: string;
  className?: string;
  side?: "top" | "right" | "bottom" | "left";
  style?: CSSProperties;
  /** Controlled menu state for seams whose port doubles as the trigger. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
  const { nodeTypes, readOnly, insert } = useWorkflowIterationActions();
  const { t } = useTranslation();
  if (readOnly) {
    return null;
  }
  return (
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger
        render={
          <Button
            type="button"
            variant="default"
            size="icon-xs"
            style={style}
            className={cn(
              "nodrag nopan rounded-full bg-blue-600 text-white shadow-sm hover:bg-blue-700",
              className,
            )}
            aria-label={label}
          />
        }
      >
        <IconPlus className="size-3" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center" side={side} className="w-44">
        {nodeTypes.map((nodeType) => {
          const Icon = getNodeMetadata(nodeType.kind).icon;
          return (
            <DropdownMenuItem
              key={nodeType.kind}
              className="gap-2 text-xs"
              onClick={() => insert(nodeType.kind, insertion)}
            >
              <Icon className="size-3.5" />
              {nodeType.label}
            </DropdownMenuItem>
          );
        })}
        {nodeTypes.length === 0 && (
          <p className="px-2 py-3 text-xs text-muted-foreground">
            {t("settings.workflow.iteration.noSupportedNodes")}
          </p>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
