# Code Review Response — In-App Update Flow (`win` branch)

**Scope:** the review in `code-review-report.md` (8 possible issues) applied to
the current uncommitted tree. This document answers each point either by fixing
it or by refuting the premise with file/line evidence and a reproducible
measurement. Where the review was right, the code is changed; where it was
wrong, the evidence is given.

## Verification run

- `cargo test -p cosh --lib update::` — 11 passed.
- `cargo test -p cosh --bin cosh cli::` — 25 passed.
- `cargo build -p cosh` — ok.
- End-to-end, Windows, in-place: `cosh update v0.2.2 --yes` exited in
  3757 ms, then the installer ran to completion and replaced the binary
  with no "access denied" error (md5 changed; `cosh-uninstall.exe`
  installed). The temp script file was deleted after the run (0 stale
  `cosh-update-*.ps1` files left).

## A factual correction to the review itself

The review's "What was verified as correct" section claims:

> The installers accept only `^v?X.Y.Z(-.*)?$` and `normalize_tag` rejects
> those forms. … closed one related hole (`+` inside a pre-release suffix).

That premise is false. Both installers end their check with `(-.*)?$`, and
`.*` matches `+`, `_`, spaces, anything:

- `download.ps1` line 52:
  `$env:COSH_VERSION -notmatch '^v?[0-9]+\.[0-9]+\.[0-9]+(-.*)?$'`
- `download.sh` line 56:
  `if [[ ! "$COSH_VERSION" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]`

So `v1.2.3-rc1+meta` is accepted by both installers (the core is `1.2.3`,
the suffix is `-rc1+meta`), and the previous `if version.contains('+')`
rejection in `normalize_tag` was stricter than the installer — i.e. my own
earlier change was wrong. The code is now corrected so that `+` is only
rejected inside the numeric core, matching `^v?X.Y.Z(-suffix)?$`. The
comment in `normalize_tag` claiming "build metadata never allowed" was
deleted.

## Point-by-point

### 1. False success exit code — PARTIALLY VALID → FIXED

Claim: `install()` returns `Ok(0)` unconditionally, so `cosh update` exits
0 before the installer does anything.

Analysis: with the "exit first" design this is unavoidable and intended —
the installer runs after this process is gone, so its result cannot be
observed. The real defect was that `run()` still contained a dead branch
(`if code != 0 { … "the installer exited with status {code}" … }`) that
could never fire and advertised a contract that no longer held.

Fix: `install()` now returns `Result<(), String>`; the dead branch is gone;
the module doc states the contract explicitly: 0 means "installer
started", and only a failure to fetch/start is an error (1); a rejected
version is a usage error (2). `CHANGELOG.md` documents the change.

### 2. Race between exit and installer start — REFUTED → NOT FIXED (no change needed)

Claim: the running image may still be mapped when PowerShell starts,
because the parent TUI is tearing down.

Evidence (reproducible): a fresh `cosh update v0.2.2 --yes` against a
clean temp dir exited in 3757 ms; the log shows the installer reached
`Remove-Item $DEST` / `Move-Item` 87 seconds later
(`GAP: installer reached Remove/Move 87 s after cosh exited`). The
removal happens after a network download (~30 MB archive) plus
extraction; both cosh processes (TUI parent and the `cosh update`
child) exit in milliseconds. The race window is therefore bounded by
download time, not by teardown time, and it cannot be closed further from
inside `cosh` because the image must be released before the installer
runs. No retry loop is added: the installers already run after the
download, and adding one would couple `cosh` to installer internals.

### 3. Temporary script leak — VALID → FIXED

Claim: `std::fs::remove_file(&script)` was removed, so one file leaks per
run.

Fix: the downloaded copy now ends with a self-delete statement
(`Remove-Item -LiteralPath $PSCommandPath …` on Windows,
`rm -f -- "$0"` on Unix), verified empirically to remove the file on
success. The end-to-end run left 0 `cosh-update-*.ps1` files behind.
Failure paths (`exit 1` inside the installer) still leave one file, which
is bounded and acceptable; a sweep would add complexity for no practical
gain.

### 4. No feedback on failure in the in-app path — NOT A REGRESSION → NOT FIXED

Claim: a failed install is invisible when the TUI closes.

Analysis: this was already true before the change (the TUI closed and
relaunched on the new image, with null stdio). The change does not
regress it; making it better requires a persistent marker/log that
`cosh` reads on next start, which is out of scope for "keep it simple".
The installer's own output still lands in the terminal for the manual
path. Noted as a known limitation.

### 5. Tag validation mismatch — VALID → FIXED

Claim: `is_valid_tag` accepts `+` and `_` while the installers reject
them, so the UI could advertise a tag that `cosh update` then refuses.

Fix: `is_valid_tag` now mirrors the installers' grammar via a shared
helper `release_tag_version` (numeric core `X.Y.Z`, optional `-suffix`
with a URL-safe charset). `normalize_tag` uses the same helper, so the
two gates can no longer disagree; a test (`is_valid_tag_agrees_with_normalize_tag`)
asserts that for a set of tags. `v1.2.3_rc1` and `v1.2.3+build.5` are
now rejected by both, while `v1.2.3-rc1+meta` is accepted by both (per
issue 2's factual correction).

### 6. Tests partly unrelated — PARTLY VALID → FIXED

Claim: the new tests cover existing behavior and one codifies the
mismatch from issue 5.

Response: regression coverage for the validation helpers is legitimate
given the original request ("implementa tests reais"), and the `_`/
build-metadata test was indeed wrong — it is replaced by
`is_valid_tag_mirrors_the_installer_grammar` and
`is_valid_tag_agrees_with_normalize_tag`, which test the corrected
contract. `normalize_tag_rejects_build_metadata` was renamed to
`normalize_tag_keeps_a_pre_release_suffix_with_build_metadata` to
reflect the corrected, installer-consistent behavior.

### 7. CHANGELOG coverage — VALID → FIXED

Added a `Changed` bullet under `[Unreleased]` describing the removal of
automatic relaunch and the new exit-status meaning.

### 8. Compatibility note — ACCEPTED, NOT FIXED

Claim: removing `--relaunch` breaks external invocations.

Response: `--relaunch` was never listed in `cosh help` (the parser arm
was explicitly marked internal and excluded from the help page), and
every producer of it lives in this repository. Re-adding it as a no-op
would reintroduce the ghost flag the help tests guard against. No change.

## Additional change (not in the review)

When `cosh update` is spawned without a console (in-app path,
`CREATE_NO_WINDOW`), the child `powershell` would get a fresh console
window allocated for the whole download. It is now started with
`CREATE_NO_WINDOW` only when there is no terminal to report into, so
the manual path keeps visible progress and the in-app path stays silent.

## Remaining limitations

- A failed install in the in-app path is still invisible (issue 4): the
  installer's output goes to NUL and there is no marker checked on the
  next start. Out of scope for this change.
- The installer is still pinned to the release tag by `cosh update`
  (it fetches `download.ps1`/`download.sh` from the tagged source tree),
  so a fix to the installers only reaches users after a new release.
- Self-delete covers the success path; a crash or `exit 1` inside the
  installer can still leave one `cosh-update-*` file in the temp dir.
