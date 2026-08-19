import { create } from "zustand";

interface ComposerPluginSelectionState {
  /**
   * Plugin ids applied through "@" or "+", keyed by conversation (see `conversationKeyFor`).
   * The composer instance is reused across session switches rather than remounted, so this
   * state has to live outside the component or one conversation's picks would bleed into
   * another's. Keying on the session rather than the task is what keeps sibling sessions
   * under one task independent.
   */
  selectedIdsByConversation: Record<string, string[]>;
  /** Applies a plugin to a conversation's message, ignoring it if already applied. */
  addPlugin: (key: string, pluginId: string) => void;
  /** Removes a plugin from a conversation's applied set. */
  removePlugin: (key: string, pluginId: string) => void;
  /** Moves a selection to a new key (used when an optimistic session gets its real id). */
  rekey: (fromKey: string, toKey: string) => void;
}

export const useComposerPluginSelectionStore =
  create<ComposerPluginSelectionState>((set) => ({
    selectedIdsByConversation: {},
    addPlugin: (key, pluginId) =>
      set((state) => {
        const current = state.selectedIdsByConversation[key] ?? [];
        if (current.includes(pluginId)) return state;
        return {
          selectedIdsByConversation: {
            ...state.selectedIdsByConversation,
            [key]: [...current, pluginId],
          },
        };
      }),
    removePlugin: (key, pluginId) =>
      set((state) => {
        const current = state.selectedIdsByConversation[key];
        if (current === undefined) return state;
        return {
          selectedIdsByConversation: {
            ...state.selectedIdsByConversation,
            [key]: current.filter((id) => id !== pluginId),
          },
        };
      }),
    rekey: (fromKey, toKey) =>
      set((state) => {
        const selected = state.selectedIdsByConversation[fromKey];
        if (selected === undefined || fromKey === toKey) return state;
        const next = { ...state.selectedIdsByConversation };
        delete next[fromKey];
        next[toKey] = selected;
        return { selectedIdsByConversation: next };
      }),
  }));
