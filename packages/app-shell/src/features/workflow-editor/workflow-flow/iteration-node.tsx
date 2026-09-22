import { useLayoutEffect, useState } from "react";
import {
  type ControlPosition,
  Handle,
  type Node,
  NodeResizeControl,
  Position,
  ResizeControlVariant,
  useReactFlow,
  useUpdateNodeInternals,
} from "@xyflow/react";
import { useTranslation } from "react-i18next";
import {
  IconChevronDown,
  IconChevronUp,
  IconHomeFilled,
  IconStack2,
  IconTrash,
} from "@tabler/icons-react";
import {
  WORKFLOW_ITERATION_COLLAPSED_HEIGHT,
  WORKFLOW_ITERATION_COLLAPSED_WIDTH,
  WORKFLOW_ITERATION_ENTRY_HANDLE_Y,
  WORKFLOW_ITERATION_HEADER_HEIGHT,
  WORKFLOW_ITERATION_NODE_HEIGHT,
  WORKFLOW_ITERATION_NODE_WIDTH,
  type WorkflowNodeData,
} from "@ora/workflow-mock";
import { cn } from "@ora/ui";
import {
  useWorkflowNodeUnused,
  WorkflowUnusedBadge,
} from "../../workflow-node-chrome";
import { IterationInsertMenu } from "./iteration-actions";
import { useWorkflowIterationActions } from "./iteration-actions-context";

/** Matches Dify's iteration-start block: a 44px card wrapping a blue home badge. */
const ITERATION_START_SIZE = 44;
const ITERATION_START_LEFT = 24;
/** Dify-style corner affordance: a forgiving resize hit zone flush with the corner. */
const ITERATION_RESIZE_HANDLE_SIZE = 24;
/** Dify's resize glyph: one soft arc hugging the rounded corner. */
const ITERATION_RESIZE_ARC_PATH =
  "M5.19009 11.8398C8.26416 10.6196 10.7144 8.16562 11.9297 5.08904";

export interface IterationNodeFrameProps {
  id: string;
  data: WorkflowNodeData;
  selected: boolean;
  deletable?: boolean;
  isInputCandidate: boolean;
  isOutputCandidate: boolean;
  positionAbsoluteX: number;
  positionAbsoluteY: number;
  nodeKindLabel: string;
}

/** Owns the complete editor presentation of one iteration composite region. */
export function IterationNodeFrame({
  id,
  data,
  selected,
  deletable,
  isInputCandidate,
  isOutputCandidate,
  positionAbsoluteX,
  positionAbsoluteY,
  nodeKindLabel,
}: IterationNodeFrameProps) {
  const { t } = useTranslation();
  const unused = useWorkflowNodeUnused(id);
  const { deleteElements, getNode } =
    useReactFlow<Node<WorkflowNodeData, "workflow">>();
  const updateNodeInternals = useUpdateNodeInternals();
  const iterationActions = useWorkflowIterationActions();
  // The entry port doubles as the insert trigger (Dify behavior): the hover plus is
  // decorative, so the port click drives this controlled menu instead.
  const [entryMenuOpen, setEntryMenuOpen] = useState(false);
  const collapsed = data.collapsed === true;
  const memberCount =
    typeof data.regionMemberCount === "number" ? data.regionMemberCount : 0;
  const frame = getNode(id);
  // Manual resizes rewrite initialWidth/initialHeight on every gesture frame, so
  // the persisted size and the rendered frame never drift apart mid-drag.
  const expandedWidth = Math.max(
    WORKFLOW_ITERATION_NODE_WIDTH,
    frame?.initialWidth ?? WORKFLOW_ITERATION_NODE_WIDTH,
  );
  const expandedHeight = Math.max(
    WORKFLOW_ITERATION_NODE_HEIGHT,
    frame?.initialHeight ?? WORKFLOW_ITERATION_NODE_HEIGHT,
  );

  useLayoutEffect(() => {
    // The internal start handle appears only in expanded mode. Refreshing after the DOM commit
    // keeps React Flow's handle registry aligned with the presentation-only composite chrome.
    // Size changes must not re-enter updateNodeInternals: that remeasures members, which
    // expands frames again, which retoggles this effect and flickers nested canvases.
    updateNodeInternals(id);
  }, [collapsed, id, updateNodeInternals]);

  return (
    <div
      data-workflow-node=""
      data-workflow-node-id={id}
      data-x={String(Math.round(positionAbsoluteX))}
      data-y={String(Math.round(positionAbsoluteY))}
      data-workflow-iteration-frame=""
      data-collapsed={collapsed}
      aria-label={`${data.title}: ${nodeKindLabel}`}
      className={cn(
        unused && "opacity-60",
        // The frame keeps one constant background; selection only repaints the
        // border and shadow (Dify shows a green border, never a fill change).
        "group/iteration-frame relative overflow-visible rounded-2xl border bg-violet-500/[0.035] shadow-sm transition-[border-color,box-shadow]",
        selected
          ? "border-ring shadow-md ring-2 ring-ring/10"
          : "border-violet-500/40",
        isOutputCandidate && "border-ring/70 ring-2 ring-ring/10",
      )}
      style={{
        width: collapsed ? WORKFLOW_ITERATION_COLLAPSED_WIDTH : expandedWidth,
        height: collapsed
          ? WORKFLOW_ITERATION_COLLAPSED_HEIGHT
          : expandedHeight,
      }}
    >
      <Handle
        type="target"
        position={Position.Left}
        data-workflow-input={id}
        aria-label={t("settings.workflow.connectTo", { name: data.title })}
        className={cn(
          "workflow-port workflow-port-input !size-2.5 !border-0 !bg-transparent",
          isInputCandidate && "workflow-port-candidate",
        )}
        style={{ top: WORKFLOW_ITERATION_HEADER_HEIGHT / 2 }}
      />
      <Handle
        type="source"
        position={Position.Right}
        data-workflow-output={id}
        aria-label={t("settings.workflow.connectFrom", { name: data.title })}
        className={cn(
          "workflow-port workflow-port-output !size-2.5 !border-0 !bg-transparent",
          isOutputCandidate && "workflow-port-candidate",
        )}
        style={{ top: WORKFLOW_ITERATION_HEADER_HEIGHT / 2 }}
      />

      <div
        className="relative z-20 flex items-center gap-2 border-b border-violet-500/20 px-3"
        style={{ height: WORKFLOW_ITERATION_HEADER_HEIGHT }}
      >
        <span className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-violet-500/12 text-violet-700 dark:text-violet-300">
          <IconStack2 className="size-4" />
        </span>
        <span className="min-w-0 flex-1 truncate text-sm font-semibold">
          {data.title}
        </span>
        {unused && <WorkflowUnusedBadge />}
        <span className="shrink-0 rounded-md bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
          {t("settings.workflow.iteration.regionSummary", {
            total: memberCount,
          })}
        </span>
        <button
          type="button"
          className="nodrag nopan flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring"
          aria-label={t(
            collapsed
              ? "settings.workflow.iteration.expand"
              : "settings.workflow.iteration.collapse",
          )}
          title={t(
            collapsed
              ? "settings.workflow.iteration.expand"
              : "settings.workflow.iteration.collapse",
          )}
          onClick={() => {
            iterationActions.toggleCollapsed(id);
          }}
        >
          {collapsed ? (
            <IconChevronUp className="size-4" />
          ) : (
            <IconChevronDown className="size-4" />
          )}
        </button>
        {selected && deletable ? (
          <button
            type="button"
            className="nodrag nopan flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground outline-none hover:bg-destructive/10 hover:text-destructive focus-visible:ring-2 focus-visible:ring-ring"
            aria-label={t("settings.workflow.deleteNamed", {
              name: data.title,
            })}
            onClick={() => {
              void deleteElements({ nodes: [{ id }] });
            }}
          >
            <IconTrash className="size-3.5" />
          </button>
        ) : undefined}
      </div>

      {!collapsed && (
        <>
          <div
            data-workflow-iteration-region=""
            className="pointer-events-none absolute inset-x-3 bottom-3"
            style={{ top: WORKFLOW_ITERATION_HEADER_HEIGHT + 8 }}
          >
            {memberCount === 0 ? (
              <span className="pointer-events-none absolute right-3 top-2 text-[11px] text-muted-foreground">
                {t("settings.workflow.iteration.emptyHint")}
              </span>
            ) : undefined}
          </div>

          <div
            className="nodrag nopan group/iteration-start absolute z-20"
            style={{
              left: ITERATION_START_LEFT,
              top: WORKFLOW_ITERATION_ENTRY_HANDLE_Y - ITERATION_START_SIZE / 2,
            }}
          >
            <div
              data-workflow-iteration-start=""
              role="img"
              aria-label={t("settings.workflow.iteration.internalStart")}
              title={t("settings.workflow.iteration.internalStart")}
              className="relative flex size-11 items-center justify-center rounded-xl border border-border bg-card shadow-xs"
              onClick={
                iterationActions.readOnly
                  ? undefined
                  : () => setEntryMenuOpen((open) => !open)
              }
            >
              <span className="flex size-6 items-center justify-center rounded-full bg-blue-600 text-white">
                <IconHomeFilled className="size-3" aria-hidden="true" />
              </span>
              <Handle
                id="iteration-entry"
                type="source"
                position={Position.Right}
                data-workflow-iteration-entry={id}
                aria-label={t("settings.workflow.iteration.entryHandle", {
                  name: data.title,
                })}
                className="workflow-port workflow-port-output !size-2.5 !border-0 !bg-transparent"
                onClick={
                  iterationActions.readOnly
                    ? undefined
                    : (event) => {
                        // The port click must not double-toggle through the card
                        // handler that widens the entry affordance.
                        event.stopPropagation();
                        setEntryMenuOpen((open) => !open);
                      }
                }
              />
            </div>
            {/* Dify-style affordance: the blue plus stays decorative and centered on
                the entry port; the port itself opens the picker, and hover only fades
                the badge in so connection drags from the port are never blocked. */}
            <IterationInsertMenu
              insertion={{ type: "entry", iterationId: id }}
              label={t("settings.workflow.iteration.addNode")}
              open={entryMenuOpen}
              onOpenChange={setEntryMenuOpen}
              className="pointer-events-none absolute left-full top-1/2 z-10 -translate-x-1/2 -translate-y-1/2 opacity-0 transition-opacity duration-150 group-hover/iteration-start:opacity-100 focus-visible:opacity-100 data-popup-open:opacity-100"
            />
          </div>
          {!iterationActions.readOnly && (
            <NodeResizeControl
              variant={ResizeControlVariant.Handle}
              position={"bottom-right" satisfies ControlPosition}
              minWidth={WORKFLOW_ITERATION_NODE_WIDTH}
              minHeight={WORKFLOW_ITERATION_NODE_HEIGHT}
              className="group/iteration-resize"
              style={{
                left: "auto",
                top: "auto",
                right: 0,
                bottom: 0,
                width: ITERATION_RESIZE_HANDLE_SIZE,
                height: ITERATION_RESIZE_HANDLE_SIZE,
                // The built-in handle pins itself to the frame corner with a
                // translated 5px dot; a larger in-corner zone keeps the gesture
                // forgiving while staying clear of the rounded border.
                translate: "none",
                border: "none",
                backgroundColor: "transparent",
              }}
            >
              {/* Dify's affordance: a single soft arc hugging the corner, revealed
                  on frame hover or while the frame is selected. currentColor keeps the
                  glyph visible in dark mode where Dify's fixed black stroke would vanish. */}
              <svg
                aria-hidden="true"
                width={16}
                height={16}
                viewBox="0 0 16 16"
                fill="none"
                className={cn(
                  "absolute bottom-px right-px text-foreground opacity-0 transition-opacity duration-150 group-hover/iteration-frame:opacity-100",
                  selected && "opacity-100",
                )}
              >
                <path
                  d={ITERATION_RESIZE_ARC_PATH}
                  stroke="currentColor"
                  strokeOpacity="0.16"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </svg>
            </NodeResizeControl>
          )}
        </>
      )}
    </div>
  );
}
