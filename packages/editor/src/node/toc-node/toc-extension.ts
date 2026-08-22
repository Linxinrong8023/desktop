import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Node as ProsemirrorNode } from "@tiptap/pm/model";
import type { Transaction } from "@tiptap/pm/state";
import type { Editor } from "@tiptap/core";

export interface TOCItem {
  id: string;
  level: number;
  text: string;
  pos: number;
}

export interface TOCOptions {
  levels: number[];
  scrollBehavior: ScrollBehavior;
}

export const tocPluginKey = new PluginKey("tableOfContents");

// Monotonic counter appended to a timestamp base to guarantee uniqueness
let idCounter = 0;
function generateTocId(): string {
  return `toc-${Date.now().toString(36)}-${(idCounter++).toString(36)}`;
}

interface TOCPluginState {
  items: TOCItem[];
  decorations: DecorationSet;
}

function isHeading(node: ProsemirrorNode, levels: number[]): boolean {
  return node.type.name === "heading" && levels.includes(node.attrs.level);
}

/**
 * Build TOC state from scratch – used on plugin init and when state is
 * created via ProseMirrorState.create (e.g. content loaded from IndexedDB).
 */
function buildInitialState(
  doc: ProsemirrorNode,
  levels: number[],
): TOCPluginState {
  const items: TOCItem[] = [];
  const decorations: Decoration[] = [];

  doc.descendants((node, pos) => {
    if (isHeading(node, levels)) {
      const id = generateTocId();
      items.push({ id, level: node.attrs.level, text: node.textContent, pos });
      decorations.push(
        Decoration.node(
          pos,
          pos + node.nodeSize,
          { "data-toc-id": id },
          { tocId: id },
        ),
      );
    }
  });

  return {
    items,
    decorations: DecorationSet.create(doc, decorations),
  };
}

/**
 * Incrementally update TOC state by mapping old decorations through the
 * transaction.  Headings that survived the edit keep their original ID;
 * only genuinely new headings receive a fresh ID.  This avoids the
 * flickering / jumping caused by regenerating sequential IDs on every
 * keystroke.
 */
function applyTransaction(
  tr: Transaction,
  oldState: TOCPluginState,
  levels: number[],
): TOCPluginState {
  if (!tr.docChanged) {
    return oldState;
  }

  // Map surviving decorations through the transaction mapping
  const mapped = oldState.decorations.map(tr.mapping, tr.doc);

  // Build a position → id lookup from the surviving decorations
  const posToId = new Map<number, string>();
  mapped.find(0, tr.doc.content.size).forEach((deco) => {
    const id = (deco.spec as { tocId?: string })?.tocId;
    if (id) posToId.set(deco.from, id);
  });

  // Rebuild items & decorations from the current doc, reusing mapped IDs
  const items: TOCItem[] = [];
  const decorations: Decoration[] = [];

  tr.doc.descendants((node, pos) => {
    if (isHeading(node, levels)) {
      const id = posToId.get(pos) || generateTocId();
      items.push({ id, level: node.attrs.level, text: node.textContent, pos });
      decorations.push(
        Decoration.node(
          pos,
          pos + node.nodeSize,
          { "data-toc-id": id },
          { tocId: id },
        ),
      );
    }
  });

  return {
    items,
    decorations: DecorationSet.create(tr.doc, decorations),
  };
}

export const TOCExtension = Extension.create<TOCOptions>({
  name: "tableOfContents",

  addOptions() {
    return {
      levels: [1, 2, 3, 4],
      scrollBehavior: "smooth",
    };
  },

  addProseMirrorPlugins() {
    const { levels } = this.options;

    return [
      new Plugin({
        key: tocPluginKey,
        state: {
          init(_, state) {
            return buildInitialState(state.doc, levels);
          },
          apply(tr, oldPluginState) {
            return applyTransaction(tr, oldPluginState, levels);
          },
        },
        props: {
          decorations(state) {
            return this.getState(state)?.decorations;
          },
        },
      }),
    ];
  },
});

/** Read TOC items from the editor's plugin state */
export function getTOCItems(editor: Editor | null | undefined): TOCItem[] {
  if (!editor) return [];
  const pluginState = tocPluginKey.getState(editor.state);
  return pluginState?.items || [];
}

export default TOCExtension;
