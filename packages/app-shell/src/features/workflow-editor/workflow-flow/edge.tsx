import { memo } from "react";
import { useTranslation } from "react-i18next";
import {
  BaseEdge,
  EdgeLabelRenderer,
  type EdgeProps,
  getBezierPath,
} from "@xyflow/react";
import { cn } from "@ora/ui";
import { IterationInsertMenu } from "./iteration-actions";
import { useWorkflowIterationActions } from "./iteration-actions-context";
import { WORKFLOW_EDGE_CONTROL_Z_INDEX } from "./z-index";

/** Draws a selectable workflow edge with an accessible hit target and optional branch label. */
export const WorkflowFlowEdgeView = memo(function WorkflowFlowEdgeView({
  id,
  source,
  sourceHandleId,
  target,
  sourceX,
  sourceY,
  sourcePosition,
  targetX,
  targetY,
  targetPosition,
  label,
  selected,
  markerEnd,
  style,
  interactionWidth,
}: EdgeProps) {
  const { t } = useTranslation();
  const { insertionForEdge } = useWorkflowIterationActions();
  const insertion = insertionForEdge({
    id,
    source,
    sourceHandle: sourceHandleId,
    target,
  });
  const [edgePath, labelX, labelY] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });
  const edgeColor = selected
    ? "var(--ring)"
    : "color-mix(in oklch, var(--foreground) 46%, transparent)";

  return (
    <>
      <BaseEdge
        id={id}
        path={edgePath}
        markerEnd={markerEnd}
        interactionWidth={interactionWidth}
        style={{
          ...style,
          strokeWidth: selected ? 3 : 2,
          stroke: edgeColor,
        }}
      />
      {selected && (
        <g className="pointer-events-none">
          {[
            { x: sourceX, y: sourceY },
            { x: targetX, y: targetY },
          ].map((endpoint) => (
            <g key={`${endpoint.x}-${endpoint.y}`}>
              <circle
                cx={endpoint.x}
                cy={endpoint.y}
                r={8}
                fill="var(--background)"
                stroke="var(--ring)"
                strokeWidth={2}
              />
              <circle
                cx={endpoint.x}
                cy={endpoint.y}
                r={4.5}
                fill="var(--foreground)"
              />
            </g>
          ))}
        </g>
      )}
      <EdgeLabelRenderer>
        {insertion !== null && (
          <div
            className="nodrag nopan pointer-events-auto absolute"
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)`,
              // Edges that touch a parented member render above the node layer, so
              // the insert control must explicitly stack above every edge and node
              // elevation or the edge hit target swallows its clicks.
              zIndex: WORKFLOW_EDGE_CONTROL_Z_INDEX,
            }}
          >
            <IterationInsertMenu
              insertion={insertion}
              label={t("settings.workflow.iteration.insertOnEdge")}
              side="top"
            />
          </div>
        )}
        {label !== undefined && label !== null && label !== "" && (
          <div
            className={cn(
              "nodrag nopan pointer-events-none absolute text-[10px] text-muted-foreground",
              selected && "text-foreground",
            )}
            style={{
              transform: `translate(-50%, -50%) translate(${labelX}px, ${
                labelY - 14
              }px)`,
            }}
          >
            {String(label)}
          </div>
        )}
      </EdgeLabelRenderer>
    </>
  );
});
