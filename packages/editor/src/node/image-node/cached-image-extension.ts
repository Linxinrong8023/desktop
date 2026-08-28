import { Image } from "@tiptap/extension-image";
import { mergeAttributes } from "@tiptap/core";

export interface CachedImageOptions {
  HTMLAttributes: Record<string, unknown>;
  /** Optional URL formatter applied to image src before rendering (e.g. for local media servers). */
  formatUrl?: (url: string) => string;
}

export const CachedImage = Image.extend<CachedImageOptions>({
  addOptions() {
    return {
      HTMLAttributes: {},
      ...this.parent?.(),
      formatUrl: undefined,
    };
  },

  renderHTML({ HTMLAttributes }) {
    const attrs = { ...HTMLAttributes };
    if (this.options.formatUrl && attrs.src) {
      attrs.src = this.options.formatUrl(attrs.src as string);
    }
    return ["img", mergeAttributes(this.options.HTMLAttributes, attrs)];
  },
});
