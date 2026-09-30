# markit-composition — Donor Provenance Record

This crate is a controlled source transplant of the generic composition
kernel (K0) from the qianqian repository into the Markit product lineage
(Markit Issue #106, MARKIT-FOUNDATION-0). This file is the single
provenance authority for that transplant; production sources carry no
per-line copied-from comments.

## Record

```text
DONOR_REPOSITORY   = https://github.com/jnhu76/qianqian
DONOR_COMMIT       = ba545ee5927e1c19963dee85922569668a3fa023
DONOR_BRANCH       = main (donor HEAD at transplant time; tree clean)
DONOR_PATH         = crates/qianqian-composition
TRANSPLANT_DATE    = 2026-09-30
TRANSPLANT_TARGET  = crates/markit-composition (Markit, product/qianqian-base-1)
```

The donor commit above is the exact source state transplanted. To compare
or update against a later donor state, diff this crate against
`crates/qianqian-composition` at that commit.

## Transplant scope

Transplanted (the entire donor crate, faithfully):

```text
src/capability.rs        Capability, CapabilityKey, typed service storage
src/component.rs         ComponentSpec, registration errors, Discharge
src/context.rs           ActivationCtx, TeardownCtx, Binding, ResolveError
src/desired.rs           DesiredEntry, Revision, CompositionError(s)
src/diagnostic.rs        CompositionSnapshot, FiberDiagnostic, RelationDiagnostic
src/fiber.rs             Fiber internals, FiberState, Effect records
src/kernel.rs            CompositionKernel, EffectHandle, StepOutcome, DisposeVerdict
src/kernel_verify.rs     bounded K1–K6 scenario verification (cfg(kani)/test)
tests/*                  the generic oracle suites (see below)
```

Semantic authority: the donor design documents
`docs/architecture/composition-kernel-0-design.md`,
`docs/architecture/composition-kernel-0-implementation-adr.md`, and
`docs/architecture/composition-kernel.md` at the donor commit above. All
`§…` section citations, `ADR D…`/`B…` decision and bug identifiers,
"#70"/"#124"/FV-RUST-0/TLA+ campaign references, and review numbers
(5128371083, 5128815134) in this crate's sources and tests are citations of
those donor documents and the donor repository's process — they remain
there and were NOT transplanted.

## Excluded product domains

Nothing was taken from, and this crate must never depend on:

```text
qianqian-playback        qianqian-app           qianqian-audio-api
qianqian-output-wasapi   qianqian-decode-songcore
qianqian-songcore-sys    apps/headless          experiments/*
```

A post-transplant scan (task PRODUCT-BASE-1 §10) found zero production
usage of the donor media domain: the only donor-domain words in the crate
are the deny-list tokens inside the two firewall tests
(`tests/dependency_firewall.rs`, A13/A14 in `tests/adversarial_review.rs`)
and this provenance record itself.

## Local adaptations (the complete list)

Everything not listed here is byte-equivalent to the donor apart from two
mechanical, rename-determined classes: the crate-identifier rename
`qianqian[-_]composition` → `markit[-_]composition`, and the rustfmt line
reflow that shorter rename permits (a few multi-line call expressions
rejoin onto one line; e.g. `tests/common/mod.rs` `ActivationError::new`
calls, `tests/adversarial_review.rs` `DuplicateCapabilityName` matches) —
no token or assertion changes:

1. **Crate identity** — package `qianqian-composition` → `markit-composition`,
   first workspace member of the new root product workspace
   (`/Cargo.toml`); `version`/`edition` inherited from
   `[workspace.package]` (0.1.0 / 2024, as the donor workspace declared),
   plus `license = "Apache-2.0"` (new: Markit's root workspace license).
2. **`src/lib.rs`** — crate title; the firewall sentence now names markit
   crates and the research area; semantic-authority lines point here
   (donor documents stay in the donor repository).
3. **`src/diagnostic.rs`** — one doc word: "playback intent" →
   "application intent" (payload-free-diagnostics invariant unchanged).
4. **`src/kernel.rs` / `src/kernel_verify.rs`** — comments citing donor
   spec paths (`specs/composition-kernel-0*`) now name them as donor
   artifacts referenced through this file.
5. **`tests/dependency_firewall.rs`** — the donor manifest firewall,
   extended: the empty-`[dependencies]` table is now asserted literally,
   and a repository-boundary scan covers `src/**` + the manifest against
   donor media vocabulary, donor product crates, and Markit research/UI
   surfaces (`research/`, benchmark, horse/H0–H4, GPUI, Electron, Mermaid,
   KaTeX, markdown/parser).
6. **`tests/adversarial_review.rs`** — header/A13/A14 comments cite the
   donor reviews explicitly; A14 asserts markit manifest markers; two
   donor comments written in Chinese were translated to English; the
   duplicated deny-list entry "WASAPI" was deduplicated. No oracle logic
   changed.
7. **`tests/data_edge_oracles.rs`** — the fixture comment naming the
   donor's PcmSink type now describes a generic `bind(endpoint)` edge; the
   consumer fixture id `"music"` → `"writer"`. Fixture behavior unchanged.
8. **`tests/revision_quiescence_oracles.rs`** — the fixture id
   `"decoder"` → `"gateway"` (doc comments and local variables with it).
   Fixture behavior unchanged.
9. **Test-suite headers** — donor stage references ("#70 Stage N") are
   prefixed "donor issue #70" so they cannot be misread as Markit issues.

## License

The donor crate is part of qianqian, dual-licensed MIT OR Apache-2.0
(donor `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE` at the donor commit).
Markit redistributes this transplant under the Apache-2.0 option, which
Markit's repository license (`/LICENSE`, Apache-2.0) satisfies. The donor
crate has an empty dependency table, so no third-party notices are owed.

## Verification

Donor gate parity at transplant time (this repository, root workspace):
`cargo fmt --check`, `cargo test -p markit-composition`,
`cargo clippy -p markit-composition --all-targets -- -D warnings` — all
clean; research workspaces untouched (see the PR body for receipts).
