import {
  useWorkflowNodeUnused,
  WorkflowUnusedBadge,
} from "../workflow-node-chrome";
import { useLayoutEffect } from "react";
import { Handle, Position, useUpdateNodeInternals } from "@xyflow/react";
import { IconHomeFilled, IconSparkles, IconStack2 } from "@tabler/icons-react";
import { useTranslation } from "react-i18next";
import {
  WORKFLOW_ITERATION_ENTRY_HANDLE_Y,
  WORKFLOW_ITERATION_HEADER_HEIGHT,
} from "@ora/workflow-mock";
import { cn } from "@ora/ui";
import type {
  GraphWorkflowNodeState,
  WorkflowNodeData,
} from "@ora/workflow-runtime";
import { RunStatusBadge } from "./run-status-mark";
import { isNodeWorking, runStatusTone } from "./run-status-style";

/** Matches the editor's Dify-style iteration-start block (44px card, blue badge). */
const ITERATION_START_SIZE = 44;
const ITERATION_START_LEFT = 24;

interface RunOverviewIterationNodeProps {
  id: string;
  data: WorkflowNodeData;
  state: GraphWorkflowNodeState;
  focused: boolean;
  peerActive: boolean;
  kindLabel: string;
  artifactCount: number;
  startedLabel: string | null;
  finishedLabel: string | null;
}

/** Renders the frozen iteration as a real read-only parent frame in the run overview. */
export function RunOverviewIterationNode({
  id,
  data,
  state,
  focused,
  peerActive,
  kindLabel,
  artifactCount,
  startedLabel,
  finishedLabel,
}: RunOverviewIterationNodeProps) {
  const { t } = useTranslation();
  const updateNodeInternals = useUpdateNodeInternals();
  const unused = useWorkflowNodeUnused(id);
  const tone = runStatusTone(state.status);
  const hasTiming = startedLabel !== null || finishedLabel !== null;
  const memberCount =
    typeof data.regionMemberCount === "number" ? data.regionMemberCount : 0;

  useLayoutEffect(() => {
    // The run renderer previously registered only the outer source handle, which made
    // every frozen iteration-entry edge impossible to resolve after the snapshot loaded.
    updateNodeInternals(id);
  }, [id, updateNodeInternals]);

  return (
    <article
      data-workflow-run-node=""
      data-workflow-run-iteration-frame=""
      aria-label={`${data.title}: ${t(unused ? "workflowNode.unused" : tone.labelKey)}`}
      className={cn(
        "relative size-full overflow-visible rounded-2xl border bg-card/70 shadow-sm ring-1 transition-[border-color,box-shadow,ring-color]",
        unused && "opacity-60",
        tone.ring,
        focused && "shadow-md ring-2",
        state.status === "running" && "ring-sky-500/35 theater-live-breathe",
        state.status === "awaiting_input" &&
          "ring-amber-500/35 theater-live-breathe-amber",
        peerActive &&
          state.status !== "running" &&
          state.status !== "awaiting_input" &&
          "ring-sky-500/20",
      )}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!size-2 !border-0 !bg-transparent"
        style={{ top: WORKFLOW_ITERATION_HEADER_HEIGHT / 2 }}
        isConnectable={false}
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!size-2 !border-0 !bg-transparent"
        style={{ top: WORKFLOW_ITERATION_HEADER_HEIGHT / 2 }}
        isConnectable={false}
      />

      <header
        className="relative z-20 flex items-center gap-2 border-b border-border/70 px-3"
        style={{ height: WORKFLOW_ITERATION_HEADER_HEIGHT }}
      >
        <span className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-violet-500/12 text-violet-700 dark:text-violet-300">
          <IconStack2 className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex min-w-0 items-center gap-1.5">
            <span className="truncate text-sm font-semibold">{data.title}</span>
            <span className="shrink-0 rounded bg-muted px-1 py-0.5 text-[9px] text-muted-foreground">
              {kindLabel}
            </span>
          </div>
          {hasTiming ? (
            <p className="font-mono text-[9px] tabular-nums text-muted-foreground">
              {startedLabel ?? "—"}
              {" — "}
              {finishedLabel ?? "—"}
            </p>
          ) : undefined}
        </div>
        <span className="shrink-0 text-[9px] text-muted-foreground">
          {t("workflowRun.overview.regionSummary", { total: memberCount })}
        </span>
        {artifactCount > 0 ? (
          <span
            className="inline-flex shrink-0 items-center gap-0.5 rounded px-1 py-0.5 text-[9px] font-medium text-muted-foreground"
            title={t("workflowRun.artifacts.countBadge", {
              count: artifactCount,
            })}
          >
            <IconSparkles className="size-3" aria-hidden />
            <span className="tabular-nums">{artifactCount}</span>
          </span>
        ) : undefined}
        {unused ? (
          <WorkflowUnusedBadge />
        ) : (
          <RunStatusBadge
            status={state.status}
            live={isNodeWorking(state.status)}
            className="px-1.5 py-0 text-[9px]"
          />
        )}
      </header>

      <div
        data-workflow-run-iteration-start=""
        role="img"
        aria-label={t("workflowRun.overview.internalStart")}
        className="absolute z-20 flex items-center justify-center rounded-xl border border-border bg-card shadow-xs"
        style={{
          left: ITERATION_START_LEFT,
          top: WORKFLOW_ITERATION_ENTRY_HANDLE_Y - ITERATION_START_SIZE / 2,
          width: ITERATION_START_SIZE,
          height: ITERATION_START_SIZE,
        }}
      >
        <span className="flex size-6 items-center justify-center rounded-full bg-blue-600 text-white">
          <IconHomeFilled className="size-3" aria-hidden="true" />
        </span>
        <Handle
          id="iteration-entry"
          type="source"
          position={Position.Right}
          className="!size-2 !border-0 !bg-transparent"
          isConnectable={false}
        />
      </div>
    </article>
  );
}
