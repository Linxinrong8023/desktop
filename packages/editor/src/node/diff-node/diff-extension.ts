import { Extension } from "@tiptap/core";

export type DiffStatus = "added" | "removed";

export interface DiffOptions {
  types: string[];
}

/**
 * DiffExtension adds a `diffStatus` attribute to all block-level nodes.
 * This is used to highlight nodes with diff colors in history view.
 *
 * Usage:
 * - diffStatus: "added" - green background (new content)
 * - diffStatus: "removed" - red background (deleted content)
 */
export const DiffExtension = Extension.create<DiffOptions>({
  name: "diff",

  addOptions() {
    return {
      types: [
        "paragraph",
        "heading",
        "bulletList",
        "orderedList",
        "listItem",
        "taskList",
        "taskItem",
        "blockquote",
        "codeBlock",
        "horizontalRule",
        "image",
        "video",
        "table",
        "tableRow",
        "tableCell",
        "tableHeader",
      ],
    };
  },

  addGlobalAttributes() {
    return [
      {
        types: this.options.types,
        attributes: {
          diffStatus: {
            default: null,
            parseHTML: (element) => element.getAttribute("data-diff-status"),
            renderHTML: (attributes) => {
              if (!attributes.diffStatus) {
                return {};
              }
              return {
                "data-diff-status": attributes.diffStatus,
                class: `diff-${attributes.diffStatus}`,
              };
            },
          },
        },
      },
    ];
  },
});

export default DiffExtension;
