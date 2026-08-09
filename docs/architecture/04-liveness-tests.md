# Issue 04 — The liveness orchestration has no tests

Date: 2026-08-09
Recommendation strength: Worth exploring

## Explanation of the issue

`LivenessService` (`src-tauri/src/liveness/service.rs`) runs the poll loop and the
signal loop. It is the most failure-prone module in the app. It has zero tests.

The constructor takes four collaborators and a channel:

- `DynServiceRepository`
- `Box<dyn ServiceWatcher>`
- `Arc<RwLock<ServiceCache>>`
- `Arc<dyn EventSink>`
- `mpsc::Receiver<WatcherSignal>`

Building a harness by hand is annoying. So the tests were never written.

The cache diff functions (`liveness/cache.rs`) are tested well. The call sites are not.

Untested behaviors:

| Behavior                                             | Tested today |
| ---------------------------------------------------- | ------------ |
| Cache diff logic (apply_states, apply_full_snapshot) | yes          |
| Initial population emits no events                   | no           |
| Status signal → query → event                        | no           |
| Config signal → query → event                        | no           |
| Database signal → full refresh                       | no           |
| Set mismatch → full refresh recursion                | no           |
| Poll loop retries after an error                     | no           |

## Why it is an issue

1. **This module has the most failure modes in the app.** The poll loop, the signal
   loop, the full-refresh recursion, and the initial-event suppression all connect here.

2. **The real bugs hide in how the pieces connect.** The pure functions are tested.
   The orchestration around them is not. There is no locality between the tested
   logic and its usage.

3. **The interface is wide, and the harness is not encapsulated.** Tests are
   expensive to write, so they do not exist.

4. **Future changes have no safety net.** New signals, new events, or new refresh
   rules can break silently.

## Possible solutions

### Option A — harness and tests (recommended)

Build a test helper. The helper assembles the collaborators with test doubles:
a mock repository, a recording event sink, and a noop watcher. Then write tests
for the behaviors in the table above.

Issue 01 makes this cheap. The async repository interface removes thread mocking
from the doubles. The harness becomes plain Tokio test code.

### Option B — narrow the constructor first

Give `LivenessService` fewer collaborators. For example, group the settings
(poll interval, initial signal) into one small config value. The harness becomes
smaller. Combine with option A.

### Option C — leave untested

Accept the risk. Not recommended. The module only grows.

## What could be done

1. Land issue 01 (the blocking bridge) first. It makes the test doubles trivial.
2. Build the harness in the tests of `liveness/service.rs` (or a test helper module).
3. Write the six tests from the table above.
4. Keep the existing cache tests as they are.
5. Run `pnpm test:rust`.

The tests also enforce the initial-population contract from issue 02.
