# AGENTS.md

## Project

WinServX — a Tauri v2 + Svelte 5 desktop app: a lightweight **Service Action Console** for Windows services (start/stop/restart the same 10-20 services, fast, non-blocking). Not an explorer/dashboard.

Source of truth (product spec): `C:\Users\batbo\Documents\vault\10_Personal\projects\Ideas\winservx\WinServX — Modern Windows Service Manager Specification.md`
Technical decisions: `...\WinServX Technical brainstorming.md` — design inspiration: `...\winservx-v2.html`

## Stack

- **Frontend**: Svelte 5, TypeScript, Vite, pnpm. Native CSS.
- **UI building blocks**: `bits-ui` + native CSS for every common component.
- **Backend**: Rust, single crate with layered modules (`src-tauri/src/domain`, `scm`, `queue`, `commands`, `state`). One crate, not a workspace.
- **Types**: Rust types + commands/events shared to TS via `tauri-specta` (`pnpm gen:bindings` → `src-tauri/gen`). Never hand-edit generated bindings; Rust is the single source of truth.

## Architecture rules

- **Fat backend, thin frontend**: Rust owns all business logic, service state, queue, SCM truth, persistence. Svelte owns only UI state (search text, row expansion, scroll, drawer open). Never put "is this service running" logic in `.svelte`.
- **State sync**: full snapshot via `get_services` on launch (awaits the liveness pipeline's first-refresh signal); granular events (`service-status-changed`, `queue-task-updated`) patch individual rows. No diffing/reconciliation layer.
- **Layering**: `domain` = pure types (no I/O/Tauri/Win32); `scm` = only layer touching `windows-sys`, wraps errors in own `ScmError`; `queue` = Tokio scheduler (`ActionService` with per-service sequential lanes, cross-service parallel) plus the `AsyncServiceRepository` bridge — the only place that calls `spawn_blocking`; `commands` = thin `#[tauri::command]` layer, only layer aware of `AppHandle`; `state` = `tauri::State` wiring. Idiomatic Rust DI = traits + constructor injection (`Arc<dyn Trait + Send + Sync>`), no DI framework.

## Key product behaviors (non-negotiable)

- Optimistic UI: actions animate/queue instantly, no confirmations, no blocking dialogs.
- Contextual row actions (only valid ones shown); per-row inline feedback; bottom drawer queue = only error surface (failed items persist until dismissed).
- Event-driven updates (SCM notifications), no manual refresh button.
- No auto-elevation; `ACCESS_DENIED` surfaces as normal queue failure.

## Commands

- `pnpm dev` / `pnpm tauri dev` — dev (bindings auto-generated via predev)
- `pnpm check` — svelte-check; `pnpm build` — full check + build
- `pnpm lint` — prettier + eslint + clippy (`-D warnings`); `pnpm format` — prettier write
- `pnpm test:rust` — cargo test; `pnpm test:web` — vitest; `pnpm validate` — everything
- Commit hooks via husky; don't commit generated bindings (`gen/`) or `dist/`.

## Code style

- Clean, segmented, maintainable, best-practice code; follow existing module conventions.
- No comments unless asked. Keep the frontend lean.
