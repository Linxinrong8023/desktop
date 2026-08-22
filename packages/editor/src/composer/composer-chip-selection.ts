import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, type EditorState } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const CHIP_NODE_TYPES = new Set(["composerFile", "promptToken"]);
const composerChipSelectionKey = new PluginKey("composerChipSelection");

/**
 * Atom chips skip native `::selection` paint. Mirror the range highlight onto
 * every chip that intersects a non-empty TextSelection so Select-All does not
 * look like holes between selected text runs.
 */
export function chipSelectionDecorations(state: EditorState): DecorationSet {
  const { selection, doc } = state;
  if (selection.empty) {
    return DecorationSet.empty;
  }

  const decorations: Decoration[] = [];
  doc.nodesBetween(selection.from, selection.to, (node, pos) => {
    if (!CHIP_NODE_TYPES.has(node.type.name)) {
      return;
    }
    decorations.push(
      Decoration.node(pos, pos + node.nodeSize, {
        class: "composer-chip-in-selection",
      }),
    );
  });
  return DecorationSet.create(doc, decorations);
}

/**
 * Keeps file and prompt chips visually inside a spanning text selection.
 */
export const ComposerChipSelection = Extension.create({
  name: "composerChipSelection",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: composerChipSelectionKey,
        props: {
          decorations(state) {
            return chipSelectionDecorations(state);
          },
        },
      }),
    ];
  },
});
