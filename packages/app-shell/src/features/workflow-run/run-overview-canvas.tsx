import {
  type MutableRefObject,
  type RefObject,
  useEffect,
  useMemo,
  useRef,
} from "react";
import { useTranslation } from "react-i18next";
import {
  Background,
  BackgroundVariant,
  type DefaultEdgeOptions,
  type Edge,
  MarkerType,
  type Node,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useViewport,
} from "@xyflow/react";
import { IconArrowsMaximize, IconMinus, IconPlus } from "@tabler/icons-react";
import {
  WORKFLOW_ITERATION_NODE_HEIGHT,
  WORKFLOW_ITERATION_NODE_WIDTH,
} from "@ora/workflow-mock";
import { Button } from "@ora/ui";
import {
  MAX_WORKFLOW_ZOOM,
  MIN_WORKFLOW_ZOOM,
} from "../workflow-node-chrome/viewport";
import { projectLoopRoundNodeStates } from "./loop-round-state";
import { createRunOverviewNodes } from "./run-overview-layout";
import { resolveOverviewFocusedId, resolveTheaterFocus } from "./run-focus";
import {
  RunOverviewNode,
  type RunOverviewNodeData,
  RunOverviewStatusProvider,
} from "./run-overview-node";
import { RunOverviewEdge } from "./run-overview-edge";
import type { GraphWorkflowRun, WorkflowArtifact } from "@ora/workflow-runtime";
import "@xyflow/react/dist/style.css";

const NODE_TYPE = "workflow" as const;
const EDGE_TYPE = "workflow" as const;
const FIT_PADDING = 0.18;
const RESIZE_FIT_DEBOUNCE_MS = 160;

const nodeTypes = { [NODE_TYPE]: RunOverviewNode };
const edgeTypes = { [EDGE_TYPE]: RunOverviewEdge };

const DEFAULT_EDGE_OPTIONS = {
  type: EDGE_TYPE,
  selectable: false,
  focusable: false,
  markerEnd: {
    type: MarkerType.ArrowClosed,
    width: 22,
    height: 22,
    markerUnits: "userSpaceOnUse",
    color: "color-mix(in oklch, var(--foreground) 40%, transparent)",
  },
} satisfies DefaultEdgeOptions;

interface RunOverviewCanvasProps {
  run: GraphWorkflowRun;
  focusedNodeId: string | null;
  onFocusNode: (nodeId: string) => void;
  /** Used for a soft per-node artifact affordance (count only). */
  artifacts?: WorkflowArtifact[];
  /**
   * Bump to re-run fitView and re-enable resize auto-fit (e.g. user clicks
   * Overview again after a manual pan/zoom).
   */
  fitRequestKey?: number;
}

/**
 * Fits on demand and on container resize until the user pan/zooms the graph.
 * Explicit fitRequest / snapshot change clears the manual lock.
 * Mode-enter and resize fits are instant — animated fitView after remount
 * reads as a top-to-bottom jump when switching from Theater.
 */
function OverviewViewportController({
  containerRef,
  snapshotId,
  fitRequestKey,
  userAdjustedRef,
}: {
  containerRef: RefObject<HTMLDivElement | null>;
  snapshotId: string;
  fitRequestKey: number;
  userAdjustedRef: MutableRefObject<boolean>;
}) {
  const { fitView } = useReactFlow();
  const suppressResizeFitRef = useRef(true);

  useEffect(() => {
    userAdjustedRef.current = false;
    // Ignore ResizeObserver callbacks that fire from the Theater→Overview
    // layout swap; those would otherwise queue a second fit right after mount.
    suppressResizeFitRef.current = true;
    const frame = requestAnimationFrame(() => {
      void fitView({ padding: FIT_PADDING, duration: 0 });
      // Allow resize auto-fit only after the enter fit has committed.
      requestAnimationFrame(() => {
        suppressResizeFitRef.current = false;
      });
    });
    return () => cancelAnimationFrame(frame);
  }, [fitRequestKey, fitView, snapshotId, userAdjustedRef]);

  useEffect(() => {
    const container = containerRef.current;
    if (container === null || typeof ResizeObserver === "undefined") {
      return;
    }
    let timer: ReturnType<typeof setTimeout> | null = null;
    const observer = new ResizeObserver(() => {
      if (userAdjustedRef.current || suppressResizeFitRef.current) {
        return;
      }
      if (timer !== null) {
        clearTimeout(timer);
      }
      timer = setTimeout(() => {
        timer = null;
        if (!userAdjustedRef.current && !suppressResizeFitRef.current) {
          void fitView({ padding: FIT_PADDING, duration: 0 });
        }
      }, RESIZE_FIT_DEBOUNCE_MS);
    });
    observer.observe(container);
    return () => {
      observer.disconnect();
      if (timer !== null) {
        clearTimeout(timer);
      }
    };
  }, [containerRef, fitView, userAdjustedRef]);

  return null;
}

/** Gives the read-only run canvas explicit zoom controls in addition to gestures. */
function RunOverviewViewportControls({ onFit }: { onFit: () => void }) {
  const { t } = useTranslation();
  const { fitView, zoomTo } = useReactFlow();
  const { zoom } = useViewport();

  return (
    <div
      className="absolute right-3 top-3 z-10 flex items-center rounded-lg border border-border/80 bg-background/95 p-px shadow-sm backdrop-blur"
      role="toolbar"
      aria-label={t("workflowRun.overview.zoomControls")}
    >
      <Button
        variant="ghost"
        size="icon-sm"
        className="size-7 rounded-md"
        aria-label={t("workflowRun.overview.zoomOut")}
        disabled={zoom <= MIN_WORKFLOW_ZOOM}
        onClick={() => {
          void zoomTo(Math.max(MIN_WORKFLOW_ZOOM, zoom - 0.1));
        }}
      >
        <IconMinus />
      </Button>
      <span className="flex h-7 w-9 items-center justify-center text-[9px] font-medium tabular-nums text-muted-foreground">
        {Math.round(zoom * 100)}%
      </span>
      <Button
        variant="ghost"
        size="icon-sm"
        className="size-7 rounded-md"
        aria-label={t("workflowRun.overview.zoomIn")}
        disabled={zoom >= MAX_WORKFLOW_ZOOM}
        onClick={() => {
          void zoomTo(Math.min(MAX_WORKFLOW_ZOOM, zoom + 0.1));
        }}
      >
        <IconPlus />
      </Button>
      <Button
        variant="ghost"
        size="icon-sm"
        className="size-7 rounded-md"
        aria-label={t("workflowRun.overview.fitView")}
        onClick={() => {
          onFit();
          void fitView({ padding: FIT_PADDING, duration: 180 });
        }}
      >
        <IconArrowsMaximize />
      </Button>
    </div>
  );
}

/**
 * Read-only React Flow overview of a frozen run snapshot + live nodeStates.
 * Clicking a node focuses it for Theater (caller switches mode).
 */
export function RunOverviewCanvas({
  run,
  focusedNodeId,
  onFocusNode,
  artifacts = [],
  fitRequestKey = 0,
}: RunOverviewCanvasProps) {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);
  const userAdjustedRef = useRef(false);
  const snapshot = run.definitionSnapshot;
  const nodeStates = useMemo(() => projectLoopRoundNodeStates(run, {}), [run]);
  const visibleRun = useMemo(() => ({ ...run, nodeStates }), [run, nodeStates]);
  const focus = useMemo(
    () => resolveTheaterFocus(visibleRun, focusedNodeId),
    [visibleRun, focusedNodeId],
  );
  // Terminal + no pin: do not paint Theater's fallback as selected —
  // Theater shows the result act for the same state.
  const overviewFocusedId = useMemo(
    () => resolveOverviewFocusedId(visibleRun, focusedNodeId),
    [visibleRun, focusedNodeId],
  );
  const artifactCountByNode = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const artifact of artifacts) {
      counts[artifact.nodeId] = (counts[artifact.nodeId] ?? 0) + 1;
    }
    return counts;
  }, [artifacts]);
  const memberCountByIteration = useMemo(() => {
    const counts = new Map<string, number>();
    for (const node of snapshot.nodes) {
      if (node.parentId !== undefined) {
        counts.set(node.parentId, (counts.get(node.parentId) ?? 0) + 1);
      }
    }
    return counts;
  }, [snapshot.nodes]);

  const nodes = useMemo((): Node<RunOverviewNodeData, "workflow">[] => {
    return createRunOverviewNodes(snapshot, nodeStates).map((node) => {
      const isIteration = node.data.kind === "iteration";
      const width = Math.max(
        WORKFLOW_ITERATION_NODE_WIDTH,
        finiteDimension(node.initialWidth, WORKFLOW_ITERATION_NODE_WIDTH),
      );
      const height = Math.max(
        WORKFLOW_ITERATION_NODE_HEIGHT,
        finiteDimension(node.initialHeight, WORKFLOW_ITERATION_NODE_HEIGHT),
      );
      return {
        ...node,
        type: NODE_TYPE,
        selectable: true,
        draggable: false,
        connectable: false,
        deletable: false,
        zIndex:
          isIteration || node.data.kind === "loop"
            ? 0
            : node.parentId === undefined
              ? 1
              : 2,
        ...(isIteration ? { style: { width, height } } : {}),
        data: {
          ...node.data,
          ...(isIteration
            ? {
                collapsed: false,
                regionMemberCount: memberCountByIteration.get(node.id) ?? 0,
              }
            : {}),
          runStatus: nodeStates[node.id]?.status ?? "idle",
        },
      };
    });
  }, [memberCountByIteration, snapshot, nodeStates]);

  const edges = useMemo((): Edge[] => {
    return snapshot.edges.map((edge) => {
      const sourceStatus = nodeStates[edge.source]?.status ?? "idle";
      const activePath = sourceStatus !== "idle";
      return {
        ...edge,
        type: EDGE_TYPE,
        selectable: false,
        focusable: false,
        reconnectable: false,
        data: { ...(edge.data ?? {}), activePath },
      };
    });
  }, [snapshot.edges, nodeStates]);

  return (
    <div
      ref={containerRef}
      className="relative min-h-0 flex-1 bg-muted/15"
      aria-label={t("workflowRun.overview.label")}
    >
      <ReactFlowProvider>
        <RunOverviewStatusProvider
          states={nodeStates}
          focusedNodeId={overviewFocusedId}
          activeNodeIds={focus.activeIds}
          artifactCountByNode={artifactCountByNode}
        >
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            defaultEdgeOptions={DEFAULT_EDGE_OPTIONS}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable
            edgesReconnectable={false}
            panOnScroll={false}
            zoomOnScroll
            zoomOnPinch
            panOnDrag
            minZoom={MIN_WORKFLOW_ZOOM}
            maxZoom={MAX_WORKFLOW_ZOOM}
            proOptions={{ hideAttribution: true }}
            onMoveEnd={(event) => {
              // Programmatic fitView reports a null event; user gestures do not.
              if (event !== null) {
                userAdjustedRef.current = true;
              }
            }}
            onNodeClick={(_event, node) => {
              onFocusNode(node.id);
            }}
            className="h-full w-full"
          >
            <OverviewViewportController
              containerRef={containerRef}
              snapshotId={snapshot.id}
              fitRequestKey={fitRequestKey}
              userAdjustedRef={userAdjustedRef}
            />
            <RunOverviewViewportControls
              onFit={() => {
                userAdjustedRef.current = false;
              }}
            />
            <Background
              id="run-overview-dots"
              variant={BackgroundVariant.Dots}
              gap={22}
              size={1.1}
              color="color-mix(in oklch, var(--foreground) 12%, transparent)"
            />
          </ReactFlow>
        </RunOverviewStatusProvider>
      </ReactFlowProvider>
      <p className="pointer-events-none absolute bottom-3 left-3 rounded-md border border-border/70 bg-background/85 px-2 py-1 text-[10px] text-muted-foreground backdrop-blur-sm">
        {t("workflowRun.overview.hint")}
      </p>
    </div>
  );
}

/** Accepts persisted dimensions only when they are positive finite numbers. */
function finiteDimension(value: number | undefined, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? value
    : fallback;
}
