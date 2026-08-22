import { create } from "zustand";

export interface ComposerFileSelection {
  path: string;
  startLine: number;
  endLine: number;
}

interface PendingFileContext {
  id: number;
  selections: ComposerFileSelection[];
}

interface ComposerFileContextState {
  /**
   * Pending explorer → composer injections keyed by `conversationKeyFor`.
   * Task-only keys would let a sibling session under the same worktree steal
   * chips after a mid-flight switch.
   */
  pendingByConversation: Record<string, PendingFileContext | undefined>;
  /** Queues a workspace-relative line range for one conversation's composer. */
  addSelection: (
    conversationKey: string,
    selection: ComposerFileSelection,
  ) => void;
  /** Removes a request only when it is the request the composer consumed. */
  consumeSelections: (conversationKey: string, requestId: number) => void;
}

let nextRequestId = 0;

/** Bridges file-explorer actions to the conversation composer without coupling the two views. */
export const useComposerFileContextStore = create<ComposerFileContextState>(
  (set) => ({
    pendingByConversation: {},
    addSelection: (conversationKey, selection) => {
      set((state) => {
        const pending = state.pendingByConversation[conversationKey];
        const existingSelections = pending?.selections ?? [];
        const alreadyQueued = existingSelections.some(
          (candidate) =>
            candidate.path === selection.path &&
            candidate.startLine === selection.startLine &&
            candidate.endLine === selection.endLine,
        );
        if (alreadyQueued) return state;
        const selections = [...existingSelections, selection];
        return {
          pendingByConversation: {
            ...state.pendingByConversation,
            [conversationKey]: { id: ++nextRequestId, selections },
          },
        };
      });
    },
    consumeSelections: (conversationKey, requestId) => {
      set((state) => {
        const pending = state.pendingByConversation[conversationKey];
        if (pending?.id !== requestId) return state;
        const pendingByConversation = { ...state.pendingByConversation };
        delete pendingByConversation[conversationKey];
        return { pendingByConversation };
      });
    },
  }),
);
