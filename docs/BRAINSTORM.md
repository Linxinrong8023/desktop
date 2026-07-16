# Brainstorm: shared UI in `@ora/app-shell`

## Goal

Put shared application UI (chat shell: sidebar + conversation + composer) in `packages/app-shell`, so both desktop and web hosts stay thin and reuse the same shell.

## Decisions

- Shared product UI lives in `@ora/app-shell`; apps are thin hosts only.
- Both desktop and web-client mount `<AppShell client={…} />`.
- Desktop transport for now: `@ora/mock-service` (same as web-client).
- Icons: `@tabler/icons-react` everywhere (drop lucide / Untitled icons).
- Design system: current shadcn `@ora/ui` APIs + tokens.

## Ownership boundaries

| Layer | Owns |
|---|---|
| `packages/app-shell` | Shared product UI + conversation state + contracts DI |
| `packages/ui` | Design-system primitives (shadcn / Base UI) |
| `apps/desktop` | Host bootstrap: CSS, Tauri later, mock transport → `AppShell` |
| `apps/web/client` | Host bootstrap: CSS, mock transport → `AppShell` |

## Status

- Migrated app-shell onto shadcn `@ora/ui` + `@tabler/icons-react`.
- Web-client and desktop both wire AppShell + mock-service.
- Verified web-client renders shell with Tabler icons.
