# Issue 03 — The frontend read model is untestable

Date: 2026-08-09
Recommendation strength: Strong

## Status

Resolved 2026-08-09: implemented Option A — pure frontend state modules + vitest.
Not committed yet.

- `lib/state/services.ts` — read-model reducers (`applySnapshot`,
  `applyStatusChanged`, `applyConfigChanged`, `applyServicesChanged`), optimistic
  start type + revert, `filterServices`. Import only generated types from
  `lib/tauri/bindings`.
- `lib/state/queue.ts` — queue reducers owning ids (`enqueue`, `settle`,
  `dismiss`), `pendingActions`, the auto-dismiss rule (`shouldAutoDismiss`,
  `scheduleSuccessDismiss`, `SUCCESS_CLEAR_MS`).
- `lib/components/ServiceTable/logic.ts` — row actions, startup options,
  state/start class mapping extracted from `ServiceTable.svelte` (template only
  now). The component moved to its own folder with co-located logic and tests.
- `App.svelte` — wiring only (~230 lines incl. styles); timers are tracked and
  cancelled on unmount.
- vitest (node environment, fake timers where needed); 36 tests over the three
  modules. `test:web` added to `validate`.

## Explanation of the issue

`src/App.svelte` is 304 lines. It mixes five responsibilities:

| Responsibility                         | Location in the file                                                |
| -------------------------------------- | ------------------------------------------------------------------- |
| Queue CRUD (add, settle, dismiss, ids) | `runAction`, `settle`, `dismiss`, `nextQueueId`                     |
| Services merge logic (deltas)          | `upsert`, `onStatusChanged`, `onConfigChanged`, `onServicesChanged` |
| Search and filtering                   | `query`, `filtered`                                                 |
| Elevation flow                         | `elevated`, `relaunching`, `onRelaunch`                             |
| Event subscription and layout          | `onMount`, `unlisteners`, template                                  |

The frontend keeps its own mirror of the backend read model. The merge logic is a
second implementation of the diffing that `liveness/cache.rs` does in Rust.

The frontend has no test runner. The project validates Rust tests and linting only
(`package.json`). The frontend logic is the only layer of the app that cannot be tested.

## Why it is an issue

1. **Zero tests exist for the frontend.** The queue drawer behavior (success clears,
   failure persists) is product-critical and untested. The delta application is untested.

2. **The merge logic duplicates backend diffing.** Two implementations of the same
   contract must stay in agreement. Any change to the event contract must be mirrored
   by hand in `App.svelte`. Nothing guards this.

3. **Logic embedded in a component cannot be tested** without a component harness.
   So it is not tested. The bugs hide in how the pieces connect.

4. **The component is hard to navigate.** Five responsibilities in one file.
   It is difficult to change one behavior without touching the others.

## Possible solutions

### Option A — extract plain TypeScript modules and add vitest (recommended)

Extract two modules. They are pure functions over plain data. They need no component
harness.

- `src/lib/state/services.ts` — applies a full snapshot and the three delta events
  (status, config, services changed) over a `ServiceInfo` array.
- `src/lib/state/queue.ts` — enqueue, settle, dismiss. It owns the ids.

`App.svelte` keeps only wiring and layout.

Add vitest as a small test runner. Add `test:web` to the scripts and to `validate`.

Benefits:

- Locality: "how does a row change" has one answer, with tests next to it.
- The delta contract becomes testable and documented by tests.
- The queue drawer behavior becomes testable.
- The component becomes readable.

### Option B — extract modules, no test runner

Do the same extraction but do not add vitest. The setup cost is smaller, but the
tests remain impossible.

### Option C — replace deltas with full snapshots

Stop patching rows in the frontend. The backend sends the full snapshot on each
change. This removes the mirror logic but loses granularity. It contradicts the
spec (granular events patch individual rows). Not recommended.

## What could be done

1. Extract `src/lib/state/services.ts` with a small API:

   - `applySnapshot(services, snapshot)`
   - `applyStatusChanged(services, event)`
   - `applyConfigChanged(services, event)`
   - `applyServicesChanged(services, event)`

2. Extract `src/lib/state/queue.ts` with a small API:

   - `enqueue(queue, item)`
   - `settle(queue, id, patch)`
   - `dismiss(queue, id)`

3. Rewire `App.svelte` to use the modules. Delete the embedded logic.
4. Add vitest. Add `test:web` to `package.json` and to `validate`.
5. Port the current manual scenarios into tests:

   - Status event updates state and pid.
   - Config event updates display name and start type.
   - Services event adds and removes rows.
   - Success items clear after the timeout.
   - Failed items persist until dismissed.

6. Run `pnpm validate`.
