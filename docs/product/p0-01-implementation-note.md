# P0-01 — Product Workspace + Incremental Document Core (closeout)

Status: implemented. Concise record of what landed, what it proves, and
what it deliberately defers. Sources of truth (architecture, roadmap,
invariants, plugin contract) are unchanged in their laws; this note
records implementation state only.

## What exists now

```text
Cargo.toml                     product workspace (mvp/gpui excluded)
crates/markit-core/            framework-independent core, ZERO dependencies
  id.rs                        DocumentId (process-wide monotonic)
  revision.rs                  DocumentRevision + Revisioned/StaleResult
  position.rs                  ByteOffset / LineNumber / SourceRange
  change.rs                    TextEdit / ChangeKind / EditResult / EditWork / EditError
  line_index.rs                incremental LineIndex + counters
  document.rs                  Document (private storage)
  selection.rs                 anchor/head + map_over_edit
  transaction.rs               EditTransaction / EditIntent / inverse
  snapshot.rs                  borrowing DocumentSnapshot
apps/markit/                   skeleton binary; NO GPUI dependency (G0 pending)
```

`markit-core` has no GPUI dependency, no platform crates, no plugin
runtime (P0 acceptance: core has no GPUI dependency — met by
construction). `apps/markit` exercises the core seams and waits for the
G0 baseline; the prototype's `gpui = "0.2.2"` pin was NOT promoted to a
product decision.

## Semantics delivered

- **Changed ranges are mutation-time facts.** Every successful mutation
  returns an `EditResult` with old/new covering ranges, per-edit detail,
  byte/line deltas. Downstream layers consume these; nothing re-derives
  the changed region by rescanning.
- **Revisions are exact and cheap.** One bump per successful mutation
  (single edit, transaction, or `replace_all`); rejected mutations bump
  nothing. `Revisioned<T>::commit(revision)` rejects stale derived
  results by construction — the seam INV-10/INV-13 build on.
- **The line index is incremental (ADR-003).** Load does the only full
  scan. A local edit drops covered newline successors, inserts new
  newline entries, and shifts the suffix. Line starts are strictly
  increasing with **no duplicate EOF sentinel**, so `line_of(len)`
  always addresses the last line — the prototype's out-of-range
  last-line bug class is removed by construction.
- **Coordinates are explicit.** Canonical source coordinate is the byte
  offset / byte range. UTF-16 stays at the platform edge; grapheme and
  display vocabulary get their own types when those layers exist.
- **Transactions are the mutation door.** Typing/paste/IME-commit/
  command all express as `EditTransaction` (intent + edits, atomic,
  one revision bump) returning an inverse — the natural undo and IME
  grouping seam (ADR-006/007). The undo stack itself is a later phase.
- **Snapshot is a version boundary, not a copy.** `DocumentSnapshot` is
  an O(1) borrowing view, coherent by the borrow checker. It is NOT the
  future plugin ABI; the versioned adapter
  (`plugin-compatibility-contract.md`) builds against these read-only
  semantics. No internal struct (`String`, `Vec`, indexes, pointers)
  crosses any public API.

## Evidence

- 73 tests green (unit + integration), `cargo clippy` clean,
  `cargo fmt` clean, no `unsafe`.
- **Differential:** randomized batteries (LineIndex-level and
  Document-level) drive thousands of edits over multibyte alphabets
  (ASCII / CJK / emoji / `\n`) and compare, after EVERY edit: text,
  line starts, line count, `line_of` at every offset `0..=len`, and
  line contents — against a full-scan oracle.
- **Work bounds (structural, not wall-clock; INV-01/08):** on 25K / 50K
  / 1M-line documents, a local edit asserts `bytes_scanned ==
  new_text.len()`, `full_rebuilds == 0`, `changed_lines == 1`; the
  million-line case passes on counters + oracle equality, so it cannot
  pass "by being fast".

## Known residual costs (owned, not hidden)

- `String` splicing is O(document bytes) per edit (suffix memmove).
- LineIndex suffix shift is O(lines after the edit), position-dependent
  (a begin-position edit touches ~all entries; a late edit few).

Both are documented in code, instrumented (`EditWork` counters), and
only a measured real-workload bottleneck reopens buffer redesign
(ADR-003).

## Explicit non-goals preserved

No Markdown parser/IR, no highlighting, no plugin runtime/Wasm/IPC/
marketplace, no scheduler/worker pool/ECS/scene graph/render loop, no
cache framework, no Rope/PieceTree, no tabs/file IO/rich UI. `mvp/gpui`
untouched.

## Documentation drift check

`roadmap.md` (P0 status note), `architecture.md` (§3 implementation
pointer), `issue-backlog.md` (P0 status note) updated; nothing was
promoted from "planned" to "implemented" beyond what this note's code
actually contains. `AGENTS.md` / `mvp-v0.1.md` unchanged (no rule or
gate changes).
