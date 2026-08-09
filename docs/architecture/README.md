# Architecture issues — index

This directory records five architecture issues found during the review of 2026-08-09.
Each issue has its own document. The review report (HTML) is not part of the repository.

| #   | Issue                                              | Strength        | Document                                                         |
| --- | -------------------------------------------------- | --------------- | ---------------------------------------------------------------- |
| 1   | The blocking bridge is missing                     | Strong          | [01-blocking-bridge.md](01-blocking-bridge.md)                   |
| 2   | The read model has two writers                     | Strong          | [02-read-model-single-writer.md](02-read-model-single-writer.md) |
| 3   | The frontend read model is untestable              | Strong          | [03-frontend-state-modules.md](03-frontend-state-modules.md)     |
| 4   | The liveness orchestration has no tests            | Worth exploring | [04-liveness-tests.md](04-liveness-tests.md)                     |
| 5   | The elevation code sits outside the platform layer | Speculative     | [05-privilege-layering.md](05-privilege-layering.md)             |

Suggested order of work:

1. Issue 01 — it has the highest leverage. It repairs four modules and unlocks issue 04.
2. Issue 02 — it touches the same files. It fits in the same session.
3. Issue 03 — the only frontend work.
4. Issue 04 — depends on issue 01 for cheap test doubles.
5. Issue 05 — small, safe, can be done at any time.

Each document has four sections:

- Explanation of the issue.
- Why it is an issue.
- Possible solutions.
- What could be done (concrete action plan).
