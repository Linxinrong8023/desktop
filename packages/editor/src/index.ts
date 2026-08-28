// --- Utils ---
export {
  cn,
  isMac,
  formatShortcutKey,
  parseShortcutKeys,
  isMarkInSchema,
  isNodeInSchema,
  isValidPosition,
  isExtensionAvailable,
  findNodeAtPosition,
  findNodePosition,
  isNodeTypeSelected,
  isAllowedUri,
  sanitizeUrl,
} from "./utils";

// --- Diff ---
export { diffJSONContent } from "./diff";
export type { DiffStatus } from "./diff";

// --- Hooks ---
export { useTiptapEditor } from "./hooks/use-tiptap-editor";
export { useIsMobile } from "./hooks/use-mobile";
export { useMenuNavigation } from "./hooks/use-menu-navigation";

// --- Icons ---
export { AlignCenterIcon } from "./icons/align-center-icon";
export { AlignJustifyIcon } from "./icons/align-justify-icon";
export { AlignLeftIcon } from "./icons/align-left-icon";
export { AlignRightIcon } from "./icons/align-right-icon";
export { ArrowLeftIcon } from "./icons/arrow-left-icon";
export { BanIcon } from "./icons/ban-icon";
export { BlockquoteIcon } from "./icons/blockquote-icon";
export { BoldIcon } from "./icons/bold-icon";
export { ChevronDownIcon } from "./icons/chevron-down-icon";
export { CloseIcon } from "./icons/close-icon";
export { CodeBlockIcon } from "./icons/code-block-icon";
export { Code2Icon } from "./icons/code2-icon";
export { CornerDownLeftIcon } from "./icons/corner-down-left-icon";
export { ExternalLinkIcon } from "./icons/external-link-icon";
export { HeadingFiveIcon } from "./icons/heading-five-icon";
export { HeadingFourIcon } from "./icons/heading-four-icon";
export { HeadingIcon } from "./icons/heading-icon";
export { HeadingOneIcon } from "./icons/heading-one-icon";
export { HeadingSixIcon } from "./icons/heading-six-icon";
export { HeadingThreeIcon } from "./icons/heading-three-icon";
export { HeadingTwoIcon } from "./icons/heading-two-icon";
export { HighlighterIcon } from "./icons/highlighter-icon";
export { ImagePlusIcon } from "./icons/image-plus-icon";
export { ItalicIcon } from "./icons/italic-icon";
export { LinkIcon } from "./icons/link-icon";
export { ListIcon } from "./icons/list-icon";
export { ListOrderedIcon } from "./icons/list-ordered-icon";
export { ListTodoIcon } from "./icons/list-todo-icon";
export { MoonStarIcon } from "./icons/moon-star-icon";
export { Redo2Icon } from "./icons/redo2-icon";
export { RestoreIcon } from "./icons/restore-icon";
export { StrikeIcon } from "./icons/strike-icon";
export { SubscriptIcon } from "./icons/subscript-icon";
export { SunIcon } from "./icons/sun-icon";
export { SuperscriptIcon } from "./icons/superscript-icon";
export { TrashIcon } from "./icons/trash-icon";
export { UnderlineIcon } from "./icons/underline-icon";
export { Undo2Icon } from "./icons/undo2-icon";

// --- Primitives ---
export * from "./primitive/badge";
export * from "./primitive/button";
export * from "./primitive/card";
export * from "./primitive/dropdown-menu";
export * from "./primitive/input";
export * from "./primitive/popover";
export * from "./primitive/separator";
export * from "./primitive/spacer";
export * from "./primitive/toolbar";
export * from "./primitive/tooltip";

// --- Nodes ---
export * from "./node/diff-node";
export * from "./node/image-upload-node";
export * from "./node/toc-node";
export * from "./node/video-node";
export { CachedImage } from "./node/image-node/cached-image-extension";
export type { CachedImageOptions } from "./node/image-node/cached-image-extension";
export { HorizontalRule } from "./node/horizontal-rule-node/horizontal-rule-node-extension";

// --- UI Components (explicit to avoid name conflicts across modules) ---
export { BlockquoteButton } from "./ui/blockquote-button/blockquote-button";
export { useBlockquote } from "./ui/blockquote-button/use-blockquote";

export { CodeBlockButton } from "./ui/code-block-button/code-block-button";
export { useCodeBlock } from "./ui/code-block-button/use-code-block";

export { ColorHighlightButton } from "./ui/color-highlight-button/color-highlight-button";
export { useColorHighlight } from "./ui/color-highlight-button/use-color-highlight";

export {
  ColorHighlightPopover,
  ColorHighlightPopoverContent,
  ColorHighlightPopoverButton,
} from "./ui/color-highlight-popover/color-highlight-popover";

export { HeadingButton } from "./ui/heading-button/heading-button";
export { useHeading } from "./ui/heading-button/use-heading";

export { HeadingDropdownMenu } from "./ui/heading-dropdown-menu/heading-dropdown-menu";
export { useHeadingDropdownMenu } from "./ui/heading-dropdown-menu/use-heading-dropdown-menu";

export {
  LinkPopover,
  LinkContent,
  LinkButton,
} from "./ui/link-popover/link-popover";
export { useLinkPopover } from "./ui/link-popover/use-link-popover";

export { ListButton } from "./ui/list-button/list-button";
export { useList } from "./ui/list-button/use-list";

export { ListDropdownMenu } from "./ui/list-dropdown-menu/list-dropdown-menu";
export { useListDropdownMenu } from "./ui/list-dropdown-menu/use-list-dropdown-menu";

export { MarkButton } from "./ui/mark-button/mark-button";
export { useMark } from "./ui/mark-button/use-mark";

export { TextAlignButton } from "./ui/text-align-button/text-align-button";
export { useTextAlign } from "./ui/text-align-button/use-text-align";

export { UndoRedoButton } from "./ui/undo-redo-button/undo-redo-button";
export { useUndoRedo } from "./ui/undo-redo-button/use-undo-redo";

// --- Editor ---
export { ThemeToggle } from "./editor/theme-toggle";

// --- Composer preset ---
export {
  COMPOSER_CAPABILITIES,
  createComposerExtensions,
} from "./composer/create-composer-extensions";
export type {
  ComposerExtensionOptions,
  ComposerFeatureSlot,
  ComposerPlaceholderProps,
} from "./composer/create-composer-extensions";
export {
  documentPlainText,
  plainTextToComposerContent,
} from "./composer/composer-plain-text";
export { PromptToken } from "./composer/prompt-token";
export type { PromptTokenKind } from "./composer/prompt-token";
export { ComposerNewline } from "./composer/composer-newline";
export { ComposerLink } from "./composer/composer-link";
export { ComposerFile } from "./composer/composer-file";
export type { ComposerFileAttrs } from "./composer/composer-file";
