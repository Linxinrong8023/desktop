import { useCallback, useRef, useState } from "react";
import type {
  WorkflowHistoryEvent,
  WorkflowHistoryMeta,
  WorkflowHistorySnapshot,
  WorkflowHistoryState,
} from "./workflow-history";
import {
  captureWorkflowHistorySnapshot,
  commitWorkflowHistory,
  createWorkflowHistoryState,
  redoWorkflowHistory,
  restoreWorkflowHistorySnapshot,
  undoWorkflowHistory,
  workflowHistoryFingerprint,
} from "./workflow-history";
import type { DemoWorkflow } from "@ora/workflow-mock";

interface PendingWorkflowTransaction {
  before: WorkflowHistorySnapshot;
  event: WorkflowHistoryEvent;
  meta?: WorkflowHistoryMeta;
}

type WorkflowHistoryDirection = "past" | "future";
const HISTORY_GROUP_WINDOW_MS = 500;

/** Provides session-scoped workflow history while keeping the canvas state controlled by React. */
export function useWorkflowHistory(
  onRestore: (snapshot: WorkflowHistorySnapshot) => void,
) {
  const historyRef = useRef<WorkflowHistoryState>(createWorkflowHistoryState());
  const currentSnapshotRef = useRef<WorkflowHistorySnapshot | null>(null);
  const currentEventRef = useRef<WorkflowHistoryEvent | null>(null);
  const currentMetaRef = useRef<WorkflowHistoryMeta | undefined>(undefined);
  const transactionRef = useRef<PendingWorkflowTransaction | null>(null);
  const activeGroupRef = useRef<string | null>(null);
  const activeGroupAtRef = useRef(0);
  const [published, setPublished] = useState(() => ({
    canUndo: false,
    canRedo: false,
    past: [] as WorkflowHistoryState["past"],
    future: [] as WorkflowHistoryState["future"],
    currentEvent: null as WorkflowHistoryEvent | null,
    currentMeta: undefined as WorkflowHistoryMeta | undefined,
    currentSnapshot: null as WorkflowHistorySnapshot | null,
  }));

  /** Publishes stack changes to the toolbar and history popover. */
  const notify = useCallback(() => {
    const history = historyRef.current;
    setPublished({
      canUndo: history.past.length > 0,
      canRedo: history.future.length > 0,
      past: history.past,
      future: history.future,
      currentEvent: currentEventRef.current,
      currentMeta: currentMetaRef.current,
      currentSnapshot: currentSnapshotRef.current,
    });
  }, []);

  /** Replaces the session baseline when a workflow is hydrated or switched. */
  const reset = useCallback(
    (workflow: DemoWorkflow): void => {
      historyRef.current = createWorkflowHistoryState();
      currentSnapshotRef.current = captureWorkflowHistorySnapshot(workflow);
      currentEventRef.current = null;
      currentMetaRef.current = undefined;
      transactionRef.current = null;
      activeGroupRef.current = null;
      activeGroupAtRef.current = 0;
      notify();
    },
    [notify],
  );

  /** Commits a completed discrete edit and clears redo history when it changes content. */
  const record = useCallback(
    (
      before: WorkflowHistorySnapshot,
      after: WorkflowHistorySnapshot,
      event: WorkflowHistoryEvent,
      meta?: WorkflowHistoryMeta,
      group?: string,
    ): void => {
      const beforeFingerprint = workflowHistoryFingerprint(before);
      const afterFingerprint = workflowHistoryFingerprint(after);
      if (beforeFingerprint === afterFingerprint) {
        return;
      }
      // Text inputs report one update per keystroke. Reusing the first step for
      // a focused field keeps undo aligned with a user intent instead of a keypress.
      if (
        group !== undefined &&
        activeGroupRef.current === group &&
        Date.now() - activeGroupAtRef.current <= HISTORY_GROUP_WINDOW_MS &&
        historyRef.current.past.length > 0
      ) {
        currentSnapshotRef.current = after;
        currentEventRef.current = event;
        currentMetaRef.current = meta;
        notify();
        return;
      }
      const nextState = commitWorkflowHistory(
        historyRef.current,
        before,
        after,
        event,
        meta,
      );
      historyRef.current = nextState;
      activeGroupRef.current = group ?? null;
      activeGroupAtRef.current = group === undefined ? 0 : Date.now();
      currentSnapshotRef.current = after;
      currentEventRef.current = event;
      currentMetaRef.current = meta;
      notify();
    },
    [notify],
  );

  /** Starts a transaction used for drag and text edits that emit many intermediate states. */
  const beginTransaction = useCallback(
    (
      before: WorkflowHistorySnapshot,
      event: WorkflowHistoryEvent,
      meta?: WorkflowHistoryMeta,
    ): void => {
      transactionRef.current = { before, event, meta };
      activeGroupRef.current = null;
      activeGroupAtRef.current = 0;
    },
    [],
  );

  /** Commits the pending transaction as one history step when its final content differs. */
  const commitTransaction = useCallback(
    (after: WorkflowHistorySnapshot): void => {
      const transaction = transactionRef.current;
      transactionRef.current = null;
      if (transaction === null) {
        return;
      }
      record(transaction.before, after, transaction.event, transaction.meta);
    },
    [record],
  );

  /** Ends a pending transaction without adding a history step. */
  const cancelTransaction = useCallback((): void => {
    transactionRef.current = null;
    activeGroupRef.current = null;
    activeGroupAtRef.current = 0;
  }, []);

  /** Ends the current coalescing window when a text editor loses focus. */
  const endGroup = useCallback((): void => {
    activeGroupRef.current = null;
    activeGroupAtRef.current = 0;
  }, []);

  /** Restores one previous semantic snapshot and moves the current state into redo history. */
  const undo = useCallback(
    (workflow: DemoWorkflow): boolean => {
      const current = captureWorkflowHistorySnapshot(workflow);
      const result = undoWorkflowHistory(historyRef.current, current);
      if (result.snapshot === null) {
        return false;
      }
      historyRef.current = result.state;
      activeGroupRef.current = null;
      activeGroupAtRef.current = 0;
      currentSnapshotRef.current = result.snapshot;
      currentEventRef.current = result.state.past.at(-1)?.event ?? null;
      currentMetaRef.current = result.state.past.at(-1)?.meta;
      onRestore(result.snapshot);
      notify();
      return true;
    },
    [notify, onRestore],
  );

  /** Restores one next semantic snapshot and moves the current state into undo history. */
  const redo = useCallback(
    (workflow: DemoWorkflow): boolean => {
      const current = captureWorkflowHistorySnapshot(workflow);
      const result = redoWorkflowHistory(historyRef.current, current);
      if (result.snapshot === null) {
        return false;
      }
      historyRef.current = result.state;
      activeGroupRef.current = null;
      activeGroupAtRef.current = 0;
      currentSnapshotRef.current = result.snapshot;
      currentEventRef.current = result.state.past.at(-1)?.event ?? null;
      currentMetaRef.current = result.state.past.at(-1)?.meta;
      onRestore(result.snapshot);
      notify();
      return true;
    },
    [notify, onRestore],
  );

  /** Clears both directions while keeping the current workflow content unchanged. */
  const clear = useCallback((): void => {
    historyRef.current = createWorkflowHistoryState();
    currentEventRef.current = null;
    currentMetaRef.current = undefined;
    transactionRef.current = null;
    activeGroupRef.current = null;
    activeGroupAtRef.current = 0;
    notify();
  }, [notify]);

  /** Jumps several steps in one render so a history row can restore directly. */
  const jump = useCallback(
    (
      workflow: DemoWorkflow,
      direction: WorkflowHistoryDirection,
      steps: number,
    ): boolean => {
      if (steps <= 0) {
        return false;
      }
      let nextState = historyRef.current;
      let workingWorkflow = workflow;
      let restoredSnapshot: WorkflowHistorySnapshot | null = null;
      for (let index = 0; index < steps; index += 1) {
        const current = captureWorkflowHistorySnapshot(workingWorkflow);
        const result =
          direction === "past"
            ? undoWorkflowHistory(nextState, current)
            : redoWorkflowHistory(nextState, current);
        if (result.snapshot === null) {
          break;
        }
        nextState = result.state;
        restoredSnapshot = result.snapshot;
        workingWorkflow = restoreWorkflowHistorySnapshot(
          workingWorkflow,
          result.snapshot,
        );
      }
      if (restoredSnapshot === null) {
        return false;
      }
      historyRef.current = nextState;
      currentSnapshotRef.current = restoredSnapshot;
      currentEventRef.current = nextState.past.at(-1)?.event ?? null;
      currentMetaRef.current = nextState.past.at(-1)?.meta;
      transactionRef.current = null;
      activeGroupRef.current = null;
      activeGroupAtRef.current = 0;
      onRestore(restoredSnapshot);
      notify();
      return true;
    },
    [notify, onRestore],
  );

  return {
    ...published,
    reset,
    record,
    beginTransaction,
    commitTransaction,
    cancelTransaction,
    endGroup,
    undo,
    redo,
    jump,
    clear,
  };
}

/** Rebuilds a complete editor workflow from a semantic history snapshot. */
export function restoreWorkflowFromHistory(
  workflow: DemoWorkflow,
  snapshot: WorkflowHistorySnapshot,
): DemoWorkflow {
  return restoreWorkflowHistorySnapshot(workflow, snapshot);
}
