# P0-01 — Product Workspace + Incremental Document Core (closeout)

Status: implemented, adversarial review round fixed. Concise record of
what landed, what it proves, and what it deliberately defers. Sources of
truth (architecture, roadmap, invariants, plugin contract) are unchanged
in their laws; this note records implementation state only.

## What exists now

```text
Cargo.toml                     product workspace (mvp/gpui excluded)
crates/markit-core/            framework-independent core, ZERO dependencies
  id.rs                        DocumentId (process-wide monotonic)
  revision.rs                  DocumentRevision + DocumentVersion + Revisioned/StaleResult
  position.rs                  ByteOffset / LineNumber / SourceRange
  change.rs                    TextEdit / ChangeKind / EditResult / EditWork / EditError
  line_index.rs                incremental LineIndex + counters (crate-private)
  document.rs                  Document (private storage, NOT Clone)
  selection.rs                 anchor-head + map_over_edit
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

- **Changed regions are mutation-time facts and stay sparse.** Every
  successful mutation returns an `EditResult` whose **canonical**
  invalidation regions are the per-edit `AppliedEdit` entries, each
  carrying pre-edit and post-edit byte ranges plus line spans, so
  downstream consumers (P0-02 BlockIndex) never rescan or guess. The
  top-level `covering_old_range`/`covering_new_range` are renamed,
  documented **convenience-only** sums: for a multi-edit transaction
  with distant edits they span everything between the first and last
  edit and must never be used as the invalidation region.
- **Versions bind document identity + revision.** One revision bump per
  successful mutation (single edit, transaction, or `replace_all`);
  rejected mutations bump nothing. Derived results carry a
  `DocumentVersion` (`DocumentId` + `DocumentRevision`) and
  `Revisioned<T>::commit(version)` accepts only the exact same version —
  an older revision of the same document **and an equal numeric revision
  of a different document** are both rejected by construction. A bare
  revision is never treated as globally meaningful identity.
- **Documents are not cloneable.** `Document` deliberately does not
  implement `Clone`: a clone would duplicate `DocumentId` and revision
  while letting the copies diverge, destroying the
  `(DocumentId, DocumentRevision)`-names-one-state invariant. Pinned by
  a `compile_fail` doctest; an explicit `fork`/`duplicate` API minting a
  fresh id is the only acceptable future duplication path.
- **The line index is incremental (ADR-003) and crate-private.** Load
  does the only full scan. A local edit drops covered newline
  successors, inserts new newline entries, and shifts the suffix. Line
  starts are strictly increasing with **no duplicate EOF sentinel**, so
  `line_of(len)` always addresses the last line — the prototype's
  out-of-range last-line bug class is removed by construction. The
  index is an implementation detail, not contract: `LineIndex` /
  `LineIndexCounters` are not exported (pinned by a `compile_fail`
  doctest), and algorithmic work surfaces through `EditWork`.
- **Coordinates are explicit.** Canonical source coordinate is the byte
  offset / byte range. UTF-16 stays at the platform edge; grapheme and
  display vocabulary get their own types when those layers exist.
- **Transactions are the mutation door.** Typing/paste/IME-commit/
  command all express as `EditTransaction` (intent + edits, atomic,
  one revision bump) returning an inverse — the natural undo and IME
  grouping seam (ADR-006/007). The undo stack itself is a later phase.
- **Snapshot is a version boundary, not a copy.** `DocumentSnapshot` is
  an O(1) borrowing view, coherent by the borrow checker, exposing
  `version()` alongside id/revision. It is NOT the future plugin ABI;
  the versioned adapter (`plugin-compatibility-contract.md`) builds
  against these read-only semantics. No internal struct (`String`,
  `Vec`, indexes, pointers) crosses any public API.

## Adversarial review round (pre-merge, fixed)

An adversarial review of the first P0-01 implementation found three
merge-blocking semantic issues (the C1–C12 audit item that failed was
**C5 — identity/revision correctness**). All three were fixed before
freeze; none required redoing P0-01:

1. **Revision-only staleness (C5, blocker).** `Revisioned<T>` validated
   a bare revision, so `parse(A@0).commit(B@0)` would wrongly succeed:
   equal numeric revisions from different documents are unrelated
   states. Fixed by `DocumentVersion { document_id, revision }`;
   `Document`/`DocumentSnapshot` expose `version()`; `Revisioned` /
   `StaleResult` validate the complete version. Regressions cover
   same-document+same-revision (accept), same-document+newer
   (reject), different-document+same-revision (reject).
2. **`Document: Clone` broke version identity (C5, blocker).** Cloning
   duplicated `DocumentId` + revision while allowing divergence, so a
   version pair could name two different states. `Clone` removed;
   compile-fail pin added; no fork/duplicate API invented without a
   workload.
3. **Covering-range invalidation collapsed sparsity (INV-05/08,
   blocker for P0-02).** For a transaction with distant edits, the
   top-level old/new range was being treated as the changed region:
   two one-line edits on a 1M-line document reported ~1M changed
   lines. Fixed by making per-edit regions canonical (with pre/post
   line spans preserved for BlockIndex), renaming the covering ranges
   to explicitly convenience-only, and defining
   `EditWork.changed_lines` as the disjoint union of per-edit new line
   spans. Regression: 1M-line document, edits at line 0 and the last
   line ⇒ `changed_lines == 2`, two disjoint one-line regions, while
   the covering range spans the document (and is documented as
   unusable for invalidation).

   Principle extracted: *incrementality is not only a parser property —
   the change representation itself must preserve sparsity; upstream
   merging two small edits into one huge range defeats any downstream
   incremental algorithm.*

A fourth, non-blocking finding was also applied: the public surface was
too wide — `LineIndex`/`LineIndexCounters` were exported despite the
index being a replaceable implementation detail. They are now
crate-private; `total_len`/`cumulative_counters`/`line_end` accessors
that only existed for the old public surface were removed or moved to
test-only.

## Evidence

- 83 tests green (unit + integration + 2 compile-fail pins),
  `cargo clippy --workspace --all-targets -- -D warnings` clean,
  `cargo fmt --check` clean, no `unsafe`.
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
- **Sparsity (INV-05/08):** two distant one-line edits on a 1M-line
  document in one transaction keep `changed_lines == 2` and two
  disjoint canonical regions; same-line double edits count the shared
  line once; newline-deleting edits report the merged post-edit line.

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
