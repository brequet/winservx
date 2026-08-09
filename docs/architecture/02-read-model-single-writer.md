# Issue 02 — The read model has two writers

Date: 2026-08-09
Recommendation strength: Strong

## Explanation of the issue

`ServiceCache` (`src-tauri/src/liveness/cache.rs`) is the read model for the UI.
Two modules write to it:

1. The command `get_services` (`src-tauri/src/commands.rs`).
2. The liveness pipeline (`src-tauri/src/liveness/service.rs`).

The command does this when the cache is empty:

1. Read the cache under a read lock.
2. If it is not empty, return the snapshot.
3. If it is empty, query the SCM with `list_services`.
4. Take the write lock and check `is_empty` again.
5. Seed the cache with `apply_full_snapshot`.

The pipeline does this at the same time:

1. Refresh the cache with `apply_full_snapshot` on its own schedule.
2. Suppress events when the cache was empty (the "initial" state).

The two writers coordinate with a silent rule. The rule says: "the command seeds the
first snapshot, and the pipeline suppresses initial events and maintains the cache
afterwards." This rule lives in comments in two files. It is not in the code.

The command also copies cache logic: the double `is_empty` check and the
`apply_full_snapshot` call.

## Why it is an issue

1. **Two writers can race.** The double `is_empty` check reduces the race but does
   not remove it. Two code paths write to the same lock with no coordination.

2. **The initial-population contract is invisible.** It is a comment, not a rule.
   A future change to one writer can silently break the other.

3. **`get_services` is not thin.** AGENTS.md says the command layer is thin.
   The command contains cache reads, a direct repository call, and cache writes.

4. **The seeding is redundant.** Delete the seeding and the app still works.
   The pipeline already refreshes immediately at startup. The seeding exists only
   to save a few milliseconds on first paint.

5. **Error paths differ.** When the first refresh fails, the two writers report
   the failure differently. The frontend cannot rely on one behavior.

## Possible solutions

### Option A — single writer and readiness signal (recommended)

The pipeline owns the cache fully. It is the only writer.

The command becomes a pure read. It waits for a small readiness signal. The signal
flips once, after the first refresh. Then the command reads the cache and returns
the snapshot.

If the first refresh fails, the signal carries the failure. The command returns it.
The poll loop retries every five seconds. The frontend already has a loading state
and a retry button.

Benefits:

- One writer means no race.
- The initial-population contract becomes explicit.
- The command layer becomes truly thin.
- One error path for the first snapshot.

### Option B — keep two writers, share the seeding code

Keep two writers but extract the seeding into a shared helper. This is a smaller
change, but the contract stays implicit and the race stays.

### Option C — the frontend receives the initial snapshot as an event

Remove the seeding from `get_services` entirely. The pipeline emits an initial
snapshot event. The frontend loads from that event.

This removes the last duplication, but it changes the load flow and needs a new
event type. It is a larger frontend change.

## What could be done

1. Add the readiness signal to the liveness pipeline (a watch or notify channel).
2. Flip the signal after the first refresh in the poll loop. Flip it on success
   and on failure. The signal carries the result.
3. Rewrite `get_services`: await the signal, read the cache, return the snapshot.
4. Remove the seeding block and the double `is_empty` check from `commands.rs`.
5. Keep event suppression for the initial population. Document it next to the flip.
6. Add tests:

   - `get_services` returns the snapshot after the first refresh.
   - `get_services` returns the refresh error when the first refresh fails.
   - The poll loop retries after a failure.

7. Run `pnpm validate`.
