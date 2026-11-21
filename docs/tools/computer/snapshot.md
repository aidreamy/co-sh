# `snapshot` - read the accessibility tree

`snapshot` is how the desktop becomes **legible**: it renders an application's
accessibility tree as an indented outline (or structured JSON), one line per
element, with role, name, value and state flags. It is read-only and safe in
every mode.

```
Computer::snapshot(&self, input: &ComputerSnapshot) -> Result<SnapshotOutput, String>
snapshot(metadata, ComputerSnapshot) -> Result<SnapshotOutput, String>
```

---

## The model: scope → traverse → render

1. **Scope.** `name` or `pid` - exactly one. This is the application whose
   tree you will read.
2. **Narrow (optional).** `selector` + `nth` cut the traversal to the matched
   subtree - a single dialog, a window's group - instead of the whole app.
3. **Traverse with a budget.** `max_depth` (default 12, hard cap 40) stops
   runaway trees (web-embedded UIs can be enormous); the output reports
   `elements`, the count it rendered. A branch the budget cut is marked
   `… children not read (depth limit - raise max_depth)` - the subtree is
   unknown, not empty.
4. **Render.** `format` picks the shape:
   - `tree` (default) - a compact indented outline, one line per element;
   - `json` - structured nodes (`role`/`name`/`value`/`children`).

## Reading the outline

A line is `role name="…" value="…" description="…" stable_id="…" <tokens>
 actions=[…]`, and every quoted value can be copied straight into a selector:

```
window name="Export…" active
  text_field name="Filename" focused editable
  button name="Export" disabled actions=[press]
  button name="Export" description="Save as PDF" stable_id="pdf-btn" actions=[press,show_menu]
```

- **Tokens** are the vocabulary of [`wait`](wait.md): `disabled` is a
  `WaitState::Enabled` wait, not a blind retry. `modal` and `required`
  explain a swallowed click and a blocked submit; `active` marks the window
  that receives input.
- **`description`** is the tooltip/help text - often the only way to tell
  two same-named controls apart; **`stable_id`** is usable as the selector
  attribute `[stable_id='…']` and survives UI changes.
- **`actions`** lists the verbs the element declares: pass one of them to
  `computer_act`'s `perform_action` instead of guessing by role.

## Edges

- **A snapshot is a moment.** UI changes; re-read before acting on stale
  lines. With a `selector`, the match auto-waits up to `timeout_ms` for the
  element to exist (a hidden or minimized subtree is still snapshottable);
  for "wait until it reaches a STATE", use [`wait`](wait.md), not snapshot
  loops.
- **Empty tree, real app?** Some apps ship accessibility bridges that are
  off by default (Chromium/Electron need `--force-renderer-accessibility`).
  The error tells you - see [`errors`](errors.md).

## See also

[`screenshot`](screenshot.md) for the same app as pixels ·
`cargo run --example computer-snapshot`
