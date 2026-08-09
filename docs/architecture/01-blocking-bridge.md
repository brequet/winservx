# Issue 01 — The blocking bridge is missing

Date: 2026-08-09
Recommendation strength: Strong

## Status

Resolved 2026-08-09 (`4b4ed53`): implemented Option A — one bridge adapter.

- `queue/bridge.rs` — `run_blocking` helper + `AsyncServiceRepository` (7 async
  methods, single panic → `ServiceError::Internal` mapping).
- `ActionService` moved from `domain/` to `queue/actions.rs`; `domain` is pure again.
- All copies removed; `relaunch_as_elevated` shares `run_blocking`.
- Bridge tests: value/error passthrough and panic mapping.

## Explanation of the issue

The port `ServiceRepository` (`src-tauri/src/domain/repository.rs`) is synchronous.
Rust async code cannot call it directly. Every async caller must move the call to a
blocking thread with `spawn_blocking`.

The code repeats this pattern ten times. The ten copies use three different error shapes.

Locations of the copies:

| Module          | File                                | Number of copies |
| --------------- | ----------------------------------- | ---------------- |
| ActionService   | `src-tauri/src/domain/actions.rs`   | 4                |
| LivenessService | `src-tauri/src/liveness/service.rs` | 4                |
| Commands        | `src-tauri/src/commands.rs`         | 2                |

Each copy does the same work:

1. Clone the repository `Arc`.
2. Clone the arguments.
3. Call `spawn_blocking`.
4. Map a task panic to an error.

The copies differ in error shape:

- `actions.rs` maps a panic to `ServiceError::Internal` with its own message.
- `liveness/service.rs` uses a four-arm match: success, error, and panic arms.
- `commands.rs` uses `.and_then` or `.map_err` with `?`.

The pattern is part of the module interface. To call the repository, you must know
about threads and panics.

## Why it is an issue

1. **No locality for threading behavior.** A change to how blocking work is scheduled
   (for example, thread pool size or panic handling) requires edits in three modules.
   The knowledge is spread across the codebase.

2. **The layering rule is broken.** AGENTS.md says `domain` is pure (no I/O, no Tauri,
   no Win32). `ActionService` calls `tauri::async_runtime` inside `domain/actions.rs`.
   The rule and the code disagree.

3. **The documented `queue` layer does not exist.** AGENTS.md describes a `queue`
   module (the Tokio scheduler). The code has no `queue` module. `ActionService`
   lives in `domain/`.

4. **LivenessService is hard to test.** Every call path involves thread spawning.
   Test doubles must be wrapped in the same ceremony. Tests were never written
   (see issue 04).

5. **Error behavior is inconsistent.** Three shapes mean three different behaviors
   for the same failure. For example, a panic in `query_state` produces a different
   result shape than a panic in `refresh_all`.

## Possible solutions

### Option A — one bridge adapter (recommended)

Build one adapter. The adapter wraps any sync repository and presents an async
interface. All thread work and all panic mapping happen in this one place.

Consumers (ActionService, LivenessService, commands) simply await the interface.
They no longer know about threads.

Move ActionService into a new `queue` module. `domain` becomes pure again.
The documented layers become true.

Benefits:

- Locality: threads and panics live in one place.
- Leverage: one small adapter repairs four modules and removes about 80 lines of ceremony.
- Tests: the liveness module becomes testable without thread mocking.
- The layering rule in AGENTS.md becomes true.

### Option B — async port directly

Change the port itself to async methods. Every implementation (Windows and mocks)
becomes async.

This changes the interface for all users at once. It is a larger change, but the
seam becomes naturally async. There is no bridge to maintain.

### Option C — keep as-is

Accept the ceremony. The friction grows with every new command. Not recommended.

## What could be done

1. Define the async interface shape in a short design pass (which operations are async).
2. Implement the bridge adapter over `DynServiceRepository`. One adapter, one error mapping.
3. Switch `ActionService` to the async interface. Remove the four copies.
4. Switch `LivenessService` to the async interface. Remove the four copies.
5. Switch `commands.rs`. Remove the two copies.
6. Move `ActionService` into a new `queue` module. `domain` becomes pure.
7. Update AGENTS.md if the module layout changes.
8. Add a bridge test: a panic in the inner call maps to `ServiceError::Internal`.
9. Run `pnpm validate`.

Existing action tests keep passing. The mock repository goes through the bridge.
