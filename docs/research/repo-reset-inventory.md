# Experiment-first repository reset — reality inventory

Reset: **MARKIT-EXPERIMENT-FIRST-REPO-RESET-1** (issue #23)  
Base: `origin/master` at `33c09a4a7913601bf4adcb49ebbee647281cda3d`  
Principle: `ACTIVE only if proven necessary; otherwise ARCHIVE or DELETE`.
Git history is the archive; deletion from the active tree loses nothing.

## Authority state after this reset

```text
Product requirements       docs/PRD.md + docs/product/**   PRODUCT_TRUTH
Active research            #22 MARKIT-MARKDOWN-BENCHMARK-1  ACTIVE_RESEARCH
Archived research          Experiment 0 (#19 / PR #20)      HISTORICAL_EXPERIMENT
Superseded                 #21 MARKIT-MARKDOWN-ARCHITECTURE-1 (CLOSED)
Architecture               HOLD (docs/product/architecture.md)
Production parser          NOT YET DEFINED
Formal model               NOT YET DEFINED
Production implementation  BLOCKED
```

## Inventory

| Path (at base 33c09a4) | Current role | Authority | Needed by #22? | Action | Reason |
| ---- | ------------ | --------- | -------------- | ------ | ------ |
| `README.md` | repo entry point | stale (#19 active) | no | REWRITE | must show benchmark-first state; no old GPUI/P0-02 design as active |
| `AGENTS.md` | agent rules | stale (#19 campaign) | no | REWRITE | authority guard for #22; forbids production parser / #21 resume / Experiment 0 promotion |
| `CONTRIBUTING.md` | PR discipline | generic | no | KEEP (1 line) | evidence discipline still applies; goal line updated |
| `LICENSE`, `.editorconfig` | repo infra | none | no | KEEP_ACTIVE | non-authorial infrastructure |
| `.gitignore` | build rules | none | no | UPDATE | old results/profiles rules dropped; archive raw results rule added |
| `Cargo.toml` (root) | default workspace | none | no | DELETE | members all leave; no active production Rust crate remains; empty-shell workspace forbidden |
| `Cargo.lock` (root) | lockfile | none | no | DELETE | follows root workspace |
| `apps/markit/**` | frozen GPUI app (G0 era) | none active | no | DELETE_FROM_ACTIVE_TREE | pre-reset product implementation; git history preserves; GPUI must not become assumed frontend |
| `crates/markit-core/**` | pre-reset core; Experiment 0 measured subject | Experiment 0 only | no (explicitly NOT an implicit 7th baseline) | MOVE_TO_RESEARCH_ARCHIVE → `research/experiments/experiment-0-parser-survey/baseline/markit-core` | harness measures its public `MarkdownState` surface; required for Experiment 0 reproduction; must not become product core by existing |
| `crates/parser-survey/**` | Experiment 0 harness | Experiment 0 only | no | MOVE_TO_RESEARCH_ARCHIVE → `research/experiments/experiment-0-parser-survey/harness` | archived evidence with independent reproduction (`cargo test --manifest-path`); exits default workspace |
| `mvp/gpui/**` | GPUI feasibility prototype | none active | no | DELETE_FROM_ACTIVE_TREE | historical prototype; own lockfile irrelevant post-deletion; git history preserves |
| `bench/**` | old GPUI A2–A4 python harness | none active | no | DELETE_FROM_ACTIVE_TREE | old campaign tooling; #22 redesigns its own harness; workload semantics unreviewed for #22 |
| `tools/g0-xwin-llvm-rc.sh` | old G0 Windows build tool | none active | no | DELETE_FROM_ACTIVE_TREE | serves deleted GPUI build only |
| `workloads/**` | old editor benchmark corpora (10k–1m txt, markdown corpus) | none active | no | DELETE_FROM_ACTIVE_TREE | #22 defines its own standardized corpus; old corpora are not the #22 protocol |
| `profiles/**` | gitkeep placeholder | none | no | DELETE_FROM_ACTIVE_TREE | profiling outputs belonged to deleted harnesses |
| `results/evidence/g0/**`, `results/summary/{a2,a3,a4,g0}/**`, `results/summary/phase-a1*` | old GPUI/PocketJS campaign evidence | historical only | no | DELETE_FROM_ACTIVE_TREE | no active authority consumes them; git history preserves; pre-reset archive revision covers the era |
| `results/raw/{a2,a3,a4,g0},gpui-*,pjs-*` (untracked) | local old raw logs | none | no | DELETE (local) | stale untracked artifacts of deleted campaigns |
| `results/summary/parser-survey-*.md` | Experiment 0 curated evidence | Experiment 0 | no | MOVE_TO_RESEARCH_ARCHIVE → `.../results/summary/` | evidence travels with its experiment |
| `results/raw/parser-survey/` (untracked) | Experiment 0 raw output | Experiment 0 | no | MOVE (local, gitignored) → `.../results/raw/` | keeps archive self-contained; stays out of git |
| `docs/PRD.md` | product requirements | PRODUCT_TRUTH | no | KEEP_ACTIVE (point fixes) | canonical product authority; #19-pending wording corrected to #22 chain |
| `docs/README.md` | authority map | stale | no | REWRITE | maps #22 active / Experiment 0 archived / #21 superseded |
| `docs/product/architecture.md` | HOLD + invariants | PRODUCT_TRUTH (invariants) | no | REWRITE | HOLD retained; unlock condition now #22 → Weakness Map → algorithm → formal review → architecture; #21 gate removed |
| `docs/product/roadmap.md` | sequencing | PRODUCT_TRUTH | no | REWRITE | R-numbered chain; #21 marked SUPERSEDED; #22 NOW |
| `docs/product/mvp-v0.1.md` | V0.1 scope | PRODUCT_TRUTH | no | KEEP_ACTIVE (point fixes) | scope authority; stale #19/GPUI wording corrected |
| `docs/product/print-browser-contract.md` | print/browser contract | PRODUCT_TRUTH | no | KEEP_ACTIVE (point fixes) | output contract; GPUI-mechanism wording neutralized |
| `docs/README.md` → `docs/research/README.md` | campaign status | stale | no | REWRITE | ACTIVE #22 / ARCHIVED Experiment 0 / SUPERSEDED #21 |
| `docs/research/markdown-parser/README.md` | #19 campaign anchor | Experiment 0 | no | MOVE_TO_RESEARCH_ARCHIVE → `.../docs/markdown-parser/README.md` (+ARCHIVED banner) | belongs to the closed experiment |
| `docs/research/code-baseline.md` | #19-era old-code policy | Experiment 0 | no | MOVE_TO_RESEARCH_ARCHIVE → `.../docs/code-baseline.md` (+ARCHIVED banner) | describes a tree that no longer exists; inventory (this file) supersedes it |
| `docs/research/parser-survey-1.md` | #19 research log | Experiment 0 | no | MOVE_TO_RESEARCH_ARCHIVE → `.../docs/parser-survey-1.md` (+ARCHIVED banner) | experiment evidence |
| `docs/archive/product-reset-2026-09-16/README.md` | point-in-time boundary record | historical | no | KEEP_ACTIVE (superseded banner) | its "active authority" section is historical; banner prevents misreading |
| `research/experiments/experiment-0-parser-survey/**` | NEW archive home | HISTORICAL_EXPERIMENT | no | CREATE (this reset) | self-contained workspace: `README.md`, `harness/`, `baseline/markit-core/`, `results/`, `docs/`, own `Cargo.toml`/`Cargo.lock` |
| `research/benchmarks/markdown-ast-update/**` | #22 reserved area | ACTIVE_RESEARCH | yes | CREATE (skeleton: README + corpus/ + manifest/) | reservation only; no benchmark code, adapters, or runs in this reset |

## Who depends on what (KEEP test)

```text
Who currently depends on apps/markit, mvp, bench, tools, workloads, profiles,
old results?           -> nothing in the active tree; no active authority.
What active authority requires the old implementation? -> none.
Does #22 use any of it? -> no; #22 fixes its own baseline list and harness.
Does Experiment 0 reproduction need markit-core + parser-survey? -> yes;
   both move together into the archive (measured subject + harness).
```

## DELETED_FROM_ACTIVE_TREE summary

- `apps/markit/**` (4 files) — frozen GPUI reference app; history: pre-reset
  revision `d7837fc` and any commit before this reset.
- `mvp/**` (8 files) — GPUI feasibility prototype; same history rule.
- `bench/**` (8), `tools/**` (1), `workloads/**` (14), `profiles/**` (1) —
  old-campaign harness/corpora/tooling; same history rule.
- old-campaign `results/**` (~390 files) — GPUI/PocketJS-era evidence;
  same history rule.
- root `Cargo.toml` / `Cargo.lock` — no active production Rust workspace
  exists after the archive move; an empty shell would be misleading.

## Commit decomposition

```text
aaab826  chore: remove obsolete pre-benchmark implementation from active tree
8f772da  research: archive experiment 0 parser survey
37b379e  docs: freeze benchmark-first authority after experiment 0
<this>   docs: record experiment-first repository reset
```

## What this reset does NOT do

- does not implement any Markdown parsing or benchmark adapter;
- does not start #22 measurements;
- does not modify Experiment 0 semantics (only relocation-compatible path
  fixes: `CARGO_MANIFEST_DIR` resource lookup, literal crate version/edition,
  path dependency retarget);
- does not delete or rewrite Git history.
