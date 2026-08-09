# Issue 05 — The elevation code sits outside the platform layer

Date: 2026-08-09
Recommendation strength: Speculative

## Explanation of the issue

AGENTS.md says the `scm` layer is the only layer that touches Windows APIs:

> `scm` = only layer touching `windows-sys`, wraps errors in own `ScmError`

`privilege.rs` at the crate root imports `windows::` directly:

- `is_elevated` opens the SCM manager with `SC_MANAGER_ALL_ACCESS` and closes it.
- `relaunch_elevated` calls `ShellExecuteExW` with the `runas` verb.

The rule and the code disagree. There are two areas that touch Windows APIs:
`scm/` and the root-level `privilege.rs`.

## Why it is an issue

1. **Two different "platform layer" stories.** A reader learns the rule from
   AGENTS.md and then finds code that breaks it. The discipline is unclear.

2. **The elevation probe repeats Win32 handle patterns** from `scm/windows.rs`
   (open, check, close). The knowledge is not shared.

3. **Future Win32 code has no clear home.** The rule pushes it to the crate root.
   The platform area grows without a boundary.

4. The issue is small. The cost of the confusion is low today, but it grows
   with every new platform feature.

## Possible solutions

### Option A — move `privilege.rs` into `scm/` (recommended)

One platform area. The rule becomes true. The probe and the relaunch stay together.

The module still imports the `windows` crate, but now inside the platform layer.

### Option B — rename the platform area to `platform/`

Move the platform code under a new `platform/` directory:

- `platform/scm/`
- `platform/privilege/`

The rule becomes "only the platform layer touches Windows APIs". This is clearer
if more Win32 concerns appear later (for example, process handling or registry).

### Option C — keep as-is

Accept the exception in the rule. The confusion stays.

## What could be done

1. Move the module (either option A or B).
2. Update the module declarations in `lib.rs`.
3. Update AGENTS.md so the rule matches the code.
4. Run `pnpm validate`.

This issue is independent of the other four. It can be done at any time.
