# Remove task diff comments

## Goal

Delete the task-diff line-comment feature end to end. Do not repair the “task worktree currently unavailable” toast that appears when adding a comment.

Keep the rest of Task Changes: patch viewing, file tree, scope switching, commit, and push.

## Why the toast appears

Creating a comment loads the task worktree, re-reads the current patch, and rejects stale anchors. Clicking a gutter `+` opens that composer, so a missing worktree surfaces as `task_worktree_unavailable`.

## Decision

Remove comments completely. Do not keep a disabled UI, empty APIs, or leftover domain types.

- Keep `0003` in the catalog so existing databases do not diverge.
- Add `0009` to drop `task_diff_comments` (and its indexes/trigger). Rollback recreates the `0003` table shape.
- Remove comment-only contract types and errors, including `diffId` (only used to pin comments to a snapshot) and `task_diff_stale`.
- Keep `getTaskDiff`, `commitTaskChanges`, and `pushTaskBranch`.

## Implementation status

Done. Comment APIs, UI, domain/application/DB adapters, and public errors are removed. Existing databases drop `task_diff_comments` via migration `0009`; `0003` remains in migration history.

Follow-up polish: removed unreachable application error variants, orphan i18n, unused `fileIndex`, stale empty-state copy, dead comment-widget CSS, and docs that still described discussions.
