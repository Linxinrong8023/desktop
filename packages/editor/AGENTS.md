TipTap / ProseMirror `setContent` can call `flushSync` (React node views). That
creates two hard constraints:

1. **Do not call programmatic `setContent` / `clear` / `replaceDocument` inside
   `useLayoutEffect` (or other React commit/layout work).** Nested `flushSync`
   also fails the stderr gate.
2. **Do not let those deferred editor transactions call parent `setState`.**
   Session / draft switches often schedule TipTap updates on a microtask after
   `act(() => selectSession(…))` returns. If `onUpdate` still drives
   `onQueryChange` / attachment React state from that microtask, the update
   lands outside `act` and the suite fails.

Preferred product pattern (chat composer already follows this):

- Treat conversation-keyed **React** state (attachments, slash/@ query, menu
  dismiss) as derived from the selection key and sync it during render (or
  another path that stays inside the same `act` as the selection change).
- Keep TipTap document restore on a deferred path when needed, but suppress
  parent notify for programmatic `replaceText` / `replaceDocument` / `clear`
  (still update `dataset.composerText` for tests). User typing continues to
  emit query/doc/text callbacks normally.
- Skip no-op attachment `setState` when the parked image id list is unchanged.

Do **not** rely on sprinkling `flushComposerEffects` / extra `act` +
`Promise.resolve` in every test as the primary fix. Use those flushes only to
assert after a deferred TipTap document apply, not to hide parent `setState`
escaping `act`. When a send-failure / abandon test awaits a rejected promise,
keep that await inside `act` so restore that still updates React stays covered.
