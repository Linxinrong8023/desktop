import { InputRule } from "@tiptap/core";
import { TaskItem } from "@tiptap/extension-list";

/**
 * Kit TaskItem wraps `[ ] ` around the current block. Inside a bullet that
 * nests a checklist under a disc, which is what made `- [ ]` look like two
 * list markers. Convert the paragraph or list into a task item instead.
 */
export const ComposerTaskItem = TaskItem.extend({
  addInputRules() {
    return [
      new InputRule({
        find: /^\s*\[([ xX])\]\s$/,
        handler: ({ state, range, match, chain }) => {
          if (state.selection.$from.parent.type.name !== "paragraph") {
            return null;
          }
          const checked = match[1]?.toLowerCase() === "x";
          chain()
            .deleteRange(range)
            .toggleList("taskList", "taskItem")
            .updateAttributes("taskItem", { checked })
            .run();
        },
      }),
    ];
  },
}).configure({ nested: true });
