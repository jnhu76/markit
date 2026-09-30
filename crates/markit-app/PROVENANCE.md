# markit-app — Donor Provenance Record

This crate is a controlled source transplant of the donor's thin
composition root from the qianqian repository into the Markit product
lineage (Markit Issue #106, MARKIT-FOUNDATION-0, foundation harvest
slice). This file is the single provenance authority for that transplant;
production sources carry no per-line copied-from comments.

## Record

```text
DONOR_REPOSITORY   = https://github.com/jnhu76/qianqian
DONOR_COMMIT       = ba545ee5927e1c19963dee85922569668a3fa023
DONOR_BRANCH       = main (donor HEAD at transplant time; tree clean)
DONOR_PATH         = crates/qianqian-app
TRANSPLANT_DATE    = 2026-09-30
TRANSPLANT_TARGET  = crates/markit-app (Markit, product/qianqian-foundation-2)
```

The donor commit above is the exact source state transplanted. To compare
or update against a later donor state, diff this crate against
`crates/qianqian-app` at that commit.

## Transplant scope

Transplanted (the entire donor crate, faithfully):

```text
Cargo.toml                                manifest (see adaptations)
src/lib.rs                                MarkitApp composition root + unit tests
tests/real_resource_lifecycle/main.rs     real-resource lifecycle witnesses
```

Everything else in the donor repository was rejected for this crate:
`qianqian-playback`, `qianqian-audio-api`, `qianqian-output-wasapi`,
`qianqian-decode-songcore`, `qianqian-songcore-sys`, `apps/headless`
(product-domain code and product-specific bootstrap), and the donor
architecture/governance documents (donor evidence, not Markit authority).

## Local adaptations (the complete list)

Everything not listed here is byte-equivalent to the donor apart from the
mechanical rename classes `qianqian[-_]app` → `markit[-_]app`,
`QianqianApp` → `MarkitApp`, `qianqian[-_]composition` →
`markit[-_]composition`, the rustfmt line reflow the shorter renames
permit, and the temp-file prefix `qianqian_lifecycle_` →
`markit_lifecycle_`:

1. **Crate identity** — package `qianqian-app` → `markit-app`; path
   dependency retargeted to `markit-composition`; `version`/`edition`
   inherited from `[workspace.package]` plus
   `license = "Apache-2.0"` (the donor manifest declared neither).
2. **`src/lib.rs` `dispose()` doc comment decontaminated** — the donor
   justified the verdict-vs-snapshot authority split by citing donor
   playback-domain authority documents (ADR-PBK-002 D14.6 with the
   F6-AUTHORITY-PROMOTION-1 amendment — donor product/playback ADRs,
   not the K0 design). Markit production sources may not let a donor
   product ADR adjudicate root semantics, so the citation was removed;
   the comment now states the same ruling in the generic kernel
   vocabulary that `crates/markit-composition/src/kernel.rs` already
   uses (verdict computed from committed fiber states, never a read of
   the snapshot projection). The transplanted generic behavior is
   unchanged.
3. **`tests/real_resource_lifecycle/main.rs` header** — the donor
   disclaimer named "audio-device safety" (donor product domain); the
   same disclaimer now says "device-driver safety".
4. **Minimality-pass removals (Issue #106 slice; two donor tests)** —
   `withdrawing_the_provider_degrades_the_consumer_to_pending` (donor's
   own comment: a "plugin-authored edition" of the kernel-mediated
   withdrawal regression, already proven by the markit-composition
   oracle suites, with `revise_desired` a pure passthrough) and the
   W5 witness `root_disposal_drains_all_real_resources` (its assertions
   are the conjunction of W1 and W2, which already cover the
   provision-bearing and owner-local fiber classes at root disposal).
   No production behavior changed.

## Verification

At transplant time (this repository, root workspace):
`cargo fmt --check`, `cargo test -p markit-app`,
`cargo clippy -p markit-app --all-targets -- -D warnings` — all clean;
research workspaces untouched (see the PR body for receipts).
