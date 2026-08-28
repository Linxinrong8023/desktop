import type { ComposerFileAttrs } from "@ora/editor/composer";
import {
  fileNavigationLocation,
  type TaskChangesNavigation,
} from "./diff/task-changes-navigation-context";

/**
 * Routes a user-authored file reference chip (composer `@` mention, file-preview
 * quote, or git-diff-gutter quote) to the matching Files/Changes navigation call.
 *
 * Unlike `chat-link`'s text classification, these attrs are already structured
 * and trustworthy — no index lookup needed. Paths are workspace-relative
 * already (they come from the same explorer/search listings and diff
 * `oldPath`/`newPath` the rest of the app treats as relative), so no cwd
 * stripping is needed here either.
 *
 * Returns whether navigation happened, so callers can decide whether to
 * suppress the click's default behaviour. File-preview quotes pass their
 * inclusive `endLine` so Files/Changes can highlight the whole cited range
 * rather than only the first line. Diff quotes also pass `side` so a delete
 * range is looked up on the old side of the patch.
 */
export function navigateToFileRef(
  attrs: ComposerFileAttrs,
  navigation: TaskChangesNavigation | null,
): boolean {
  if (navigation === null || attrs.path === "") return false;
  if (attrs.kind === "directory") {
    if (navigation.openWorkspaceDirectory === undefined) return false;
    navigation.openWorkspaceDirectory(attrs.path);
    return true;
  }
  const location = fileNavigationLocation({
    line: attrs.startLine,
    endLine: attrs.endLine,
    side: attrs.diffSide,
  });
  if (attrs.origin === "diff") {
    navigation.openDiff(attrs.path, location);
    return true;
  }
  navigation.openWorkspaceFile(attrs.path, location);
  return true;
}
