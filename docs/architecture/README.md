# Architecture issues — index

This directory records five architecture issues found during the review of 2026-08-09.
Each issue has its own document. The review report (HTML) is not part of the repository.
Resolved issues carry a Status section at the top of their document.

| #   | Issue                                              | Strength        | Status   | Document                                                         |
| --- | -------------------------------------------------- | --------------- | -------- | ---------------------------------------------------------------- |
| 1   | The blocking bridge is missing                     | Strong          | Resolved | [01-blocking-bridge.md](01-blocking-bridge.md)                   |
| 2   | The read model has two writers                     | Strong          | Resolved | [02-read-model-single-writer.md](02-read-model-single-writer.md) |
| 3   | The frontend read model is untestable              | Strong          | Resolved | [03-frontend-state-modules.md](03-frontend-state-modules.md)     |
| 4   | The liveness orchestration has no tests            | Worth exploring | Resolved | [04-liveness-tests.md](04-liveness-tests.md)                     |
| 5   | The elevation code sits outside the platform layer | Speculative     | Open     | [05-privilege-layering.md](05-privilege-layering.md)             |

## Progress

| Commit          | Issue | Work                                                                  |
| --------------- | ----- | --------------------------------------------------------------------- |
| `4b4ed53`       | 01    | Blocking bridge (`queue/bridge.rs`); `ActionService` moved to `queue` |
| `58c59db`       | 02    | Single-writer read model: readiness signal, thin `get_services`       |
| `d52b3a9`       | 04    | Liveness orchestration tests (harness + six behaviors)                |
| `d580f62`       | 03    | Frontend state modules (`lib/state/*`), presentation helpers, vitest  |

## What remains

1. **Issue 05 — the elevation code sits outside the platform layer** (Speculative).
   Small and safe: move `privilege.rs` under `scm/` (or a `platform/` area), update
   `lib.rs` and AGENTS.md. Can be done at any time.

Each document has four sections: explanation of the issue, why it is an issue,
possible solutions, and a concrete action plan. Resolved documents add a Status
section on top.
