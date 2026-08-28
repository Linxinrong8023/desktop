import { create } from "zustand";
import { persist } from "zustand/middleware";
import type * as acp from "@agentclientprotocol/sdk";
import type { JSONContent } from "@tiptap/core";
import { createDebouncedJSONStorage } from "./debounced-json-storage";

/** One image parked in an unsent composer draft (memory only; not written to disk). */
export interface ParkedComposerImage {
  id: string;
  name: string;
  size: number;
  content: acp.ImageContent;
}

export interface ParkedComposerInput {
  text: string;
  images: ParkedComposerImage[];
  /**
   * TipTap document JSON so file/skill/command chips restore as chips (with
   * `kind`) instead of backtick/`$name`/`/name` plain text. Persisted with the
   * text so an app restart restores the exact reference chips; image bytes
   * stay memory only.
   */
  doc?: JSONContent;
  /**
   * Remembers that attachments existed in this process after image bytes are
   * stripped for localStorage. Image-only parks still do not survive restart.
   */
  retainedAttachments?: boolean;
}

interface ComposerInputState {
  /**
   * Unsent composer text/images keyed by `conversationKeyFor`. The composer
   * component is reused across switches, so parking here is what restores a
   * half-typed message when the user comes back to that session or draft.
   * Text and TipTap `doc` survive restarts via localStorage (chips need `doc`
   * for kind / slash tokens); image bytes stay in-process only.
   */
  byKey: Record<string, ParkedComposerInput>;
  /** Replaces the parked payload for one conversation. */
  setInput: (key: string, input: ParkedComposerInput) => void;
  /** Drops parked input when the conversation is sent or discarded. */
  clear: (key: string) => void;
  /** Drops every parked entry whose key is in the given set. */
  clearKeys: (keys: Iterable<string>) => void;
  /** Moves parked input onto a newly minted session id after first send. */
  rekey: (fromKey: string, toKey: string) => void;
  /** Test helper so cases cannot leak parked text into each other. */
  reset: () => void;
}

const EMPTY: ParkedComposerInput = { text: "", images: [] };

export const COMPOSER_INPUT_STORAGE_KEY = "ora.composer-input.v1";

/** True when leaving this conversation should keep the parked payload. */
export function composerInputHasContent(input: ParkedComposerInput): boolean {
  return (
    input.text.trim().length > 0 ||
    input.images.length > 0 ||
    input.retainedAttachments === true
  );
}

/**
 * Disk shape for one parked conversation: typed text plus a TipTap `doc` so
 * chips restore exactly. Image bytes stay in memory for the current process.
 * Restoring an empty retained stub after restart looked like a blank composer
 * with nothing to recover.
 */
function diskPark(input: unknown): ParkedComposerInput | null {
  if (
    typeof input !== "object" ||
    input === null ||
    Array.isArray(input) ||
    !("text" in input) ||
    typeof input.text !== "string" ||
    input.text.trim().length === 0
  ) {
    return null;
  }
  const parked: ParkedComposerInput = { text: input.text, images: [] };
  const doc = (input as { doc?: unknown }).doc;
  if (isParkedDoc(doc)) {
    parked.doc = doc;
  }
  return parked;
}

/** A plausible TipTap document root; deep validation happens on editor restore. */
function isParkedDoc(value: unknown): value is JSONContent {
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    (value as { type?: unknown }).type === "doc" &&
    Array.isArray((value as { content?: unknown }).content)
  );
}

/** Keeps only runtime-validated entries with typed text. */
function sanitizeParkedByKey(
  byKey: unknown,
): Record<string, ParkedComposerInput> {
  if (typeof byKey !== "object" || byKey === null || Array.isArray(byKey)) {
    return {};
  }
  const next: Record<string, ParkedComposerInput> = {};
  for (const [key, input] of Object.entries(byKey)) {
    const parked = diskPark(input);
    if (parked !== null) next[key] = parked;
  }
  return next;
}

/**
 * Parks unsent composer contents per conversation so switching sessions does
 * not throw away a half-written message. Typed text and the TipTap `doc` are
 * mirrored to localStorage (frontend only) so restart restores the exact
 * chips; attached image bytes remain in memory for the current process.
 */
export const useComposerInputStore = create<ComposerInputState>()(
  persist(
    (set) => ({
      byKey: {},
      setInput: (key, input) =>
        set((state) => {
          const previous = state.byKey[key] ?? EMPTY;
          const next: ParkedComposerInput = {
            text: input.text,
            // Store ownership must not be bypassed by a caller mutating its array.
            images: [...input.images],
            // Omit `doc` to keep a previously parked TipTap tree (text-only
            // repark from abandon must not wipe chips the composer already saved).
            ...(input.doc !== undefined
              ? { doc: input.doc }
              : previous.doc !== undefined
                ? { doc: previous.doc }
                : {}),
            ...(input.images.length > 0 || input.retainedAttachments === true
              ? { retainedAttachments: true }
              : {}),
          };
          if (
            previous.text === next.text &&
            previous.images === next.images &&
            previous.doc === next.doc &&
            previous.retainedAttachments === next.retainedAttachments
          ) {
            return state;
          }
          if (!composerInputHasContent(next)) {
            if (!(key in state.byKey)) return state;
            const byKey = { ...state.byKey };
            delete byKey[key];
            return { byKey };
          }
          return {
            byKey: {
              ...state.byKey,
              [key]: next,
            },
          };
        }),
      clear: (key) =>
        set((state) => {
          if (!(key in state.byKey)) return state;
          const byKey = { ...state.byKey };
          delete byKey[key];
          return { byKey };
        }),
      clearKeys: (keys) => {
        const removing = new Set(keys);
        if (removing.size === 0) return;
        set((state) => {
          let changed = false;
          const byKey = { ...state.byKey };
          for (const key of removing) {
            if (key in byKey) {
              delete byKey[key];
              changed = true;
            }
          }
          return changed ? { byKey } : state;
        });
      },
      rekey: (fromKey, toKey) => {
        if (fromKey === toKey) return;
        set((state) => {
          const parked = state.byKey[fromKey];
          if (parked === undefined) return state;
          const byKey = { ...state.byKey };
          delete byKey[fromKey];
          // A live destination may contain newer user input. Moving a draft must
          // never overwrite that independently parked message.
          if (!(toKey in byKey)) byKey[toKey] = parked;
          return { byKey };
        });
      },
      reset: () => set({ byKey: {} }),
    }),
    {
      name: COMPOSER_INPUT_STORAGE_KEY,
      // Keystroke parks coalesce; pagehide / visibility flush for durability.
      storage: createDebouncedJSONStorage(),
      // Never write image payloads; restart restores text, the TipTap doc, and drops attachments.
      partialize: (state) => ({
        byKey: sanitizeParkedByKey(state.byKey),
      }),
      merge: (persisted, current) => {
        const slice =
          typeof persisted === "object" && persisted !== null
            ? (persisted as Record<string, unknown>)
            : undefined;
        return {
          ...current,
          byKey: sanitizeParkedByKey(slice?.byKey),
        };
      },
    },
  ),
);
