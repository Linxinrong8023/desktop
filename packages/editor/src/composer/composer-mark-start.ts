import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

const MARK_START_META = "composerMarkStart";

/**
 * Marks are exclusive at the end so typing after `**bold**` is body text, but
 * that also leaves the caret outside the mark at the start of the run. Inherit
 * the following text's marks when the caret was already at the start of that
 * marked node (so `hello **world**` does not bold the preceding `hello `).
 */
export const ComposerMarkStartTyping = Extension.create({
  name: "composerMarkStartTyping",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("composerMarkStartTyping"),
        appendTransaction(transactions, oldState, newState) {
          if (
            !transactions.some((transaction) => transaction.docChanged) ||
            transactions.some(
              (transaction) =>
                transaction.getMeta(MARK_START_META) === true ||
                transaction.getMeta("composerMarkdownBackfill") === true ||
                transaction.getMeta("composerMarkdownRevert") === true,
            )
          ) {
            return null;
          }

          const old$ = oldState.selection.$from;
          const oldAfter = old$.nodeAfter;
          if (
            old$.nodeBefore !== null ||
            oldAfter === null ||
            !oldAfter.isText ||
            oldAfter.marks.length === 0
          ) {
            return null;
          }

          const { $from } = newState.selection;
          const before = $from.nodeBefore;
          const after = $from.nodeAfter;
          if (
            before === null ||
            after === null ||
            !before.isText ||
            !after.isText ||
            before.marks.length > 0 ||
            after.marks.length === 0
          ) {
            return null;
          }

          const from = $from.pos - before.nodeSize;
          const { tr } = newState;
          for (const mark of after.marks) {
            tr.addMark(from, $from.pos, mark);
          }
          tr.setMeta(MARK_START_META, true);
          return tr;
        },
      }),
    ];
  },
});
