import { ComposerFile } from "@ora/editor/composer";
import { ReactNodeViewRenderer } from "@tiptap/react";
import { ComposerFileChipView } from "./composer-file-chip-view";

/** App-shell file chip with Tabler type icons via React node view. */
export const AppComposerFile = ComposerFile.extend({
  addNodeView() {
    // Node is already `inline: true`; wrapper is a span so the chip stays in text flow.
    return ReactNodeViewRenderer(ComposerFileChipView, {
      as: "span",
    });
  },
});
