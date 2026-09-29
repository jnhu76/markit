# HORSE-A-V2-DIAG-98 collection provenance
collected_utc = 2026-09-29T13:27:53Z
host = E5 (E5)
cpu = Intel(R) Xeon(R) CPU E5-2666 v3 @ 2.90GHz (20
scaling cores)
kernel = Linux 7.2.5-200.fc44.x86_64
os = Fedora release 44 (Forty Four)

# --- AUTHORITATIVE executed toolchain (this block is the primary receipt;
#     corrected in the 2026-09-29 closure pass — see Erratum 1) ---
rustc = rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo = cargo 1.97.1 (c980f4866 2026-06-30)
llvm = LLVM version: 22.1.6
profile = release (opt-level 3, lto=thin, codegen-units 1, incremental=false, panic=unwind, target-cpu=default, empty RUSTFLAGS)
toolchain_binding = rust-toolchain.toml channel 1.97.1 (resolves in-workspace to the
                    rustc/cargo/LLVM values above; verified via an in-workspace
                    `rustc --version --verbose` and target/.rustc_info.json)

# --- executed-code identity (immutable; produced every sealed JSON in this directory) ---
executed_code_identity = 6b227b7
last_commit_at_collection_start = 25849a7e923fb9219a227da3ff015347f6a35249
                    (the 6b227b7 changes were still uncommitted at the moment of
                     collection; the executed code state therefore equals 6b227b7 —
                     see Erratum 2. 25849a7 alone could not have produced the sealed
                     artifacts.)
collection_binary = target/release/mdbench-horse-a-v2-diag
collection_binary_sha256 = c2b2217054f50429b644c7e66aab706147c64a20220372bb886946055d664529
                    (preserved; a rebuild of the 6b227b7 sources under the pin above
                     reproduces it bit-for-bit — the executed-toolchain proof)

# --- documentation/audit heads (do NOT change the executed identity) ---
post_audit_doc_head = 6f32664 (audit errata commit)
closure_doc_head = tip of research/98-horse-a-v2-l0-l1-diagnosis at closure
                   (6f32664 + the closure pass); the exact FINAL_PR_HEAD and
                   MERGE_COMMIT are recorded in the #98 closure receipt and in the
                   post-merge master receipt commit
execution_branch = research/98-horse-a-v2-l0-l1-diagnosis
live_master_at_start = a89a4c2723390d903edb1b38543c263e0d042f05
baseline_capsule = markit-r0-rq8-research-record-v1.tar.gz
baseline_capsule_sha256 = 156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849
commands =
  cargo build --release -p markit-mdbench-horse-a-v2-diag
  ./target/release/mdbench-horse-a-v2-diag all --out <dir>
  ./target/release/mdbench-horse-a-v2-diag decompose --out <dir>
  ./target/release/mdbench-horse-a-v2-diag manifest --out <dir>

## Erratum 1 — 2026-09-29 post-collection audit (toolchain; primary fields corrected)

The original field block of this file was captured from the host default
toolchain, not the workspace-pinned toolchain that actually built and ran the
collection binary, and read:

    rustc = rustc 1.98.1 (48a229cea 2026-09-01)   [INCORRECT capture]
    cargo = cargo 1.98.1 (797e8a9bc 2026-08-05)   [INCORRECT capture]
    llvm  = LLVM version: 22.1.8                  [INCORRECT capture]

The workspace pin (`toolchain_binding`, rust-toolchain.toml channel 1.97.1)
resolves in-workspace to **rustc/cargo 1.97.1 with LLVM 22.1.6**
(`target/.rustc_info.json`; verified with an in-workspace `rustc --version`).
The primary fields above were corrected during the closure pass; the incorrect
original values are retained here only as a labeled historical correction.
Confirmation of the executed toolchain: rebuilding the 6b227b7 sources under
the pin reproduces the preserved collection binary bit-for-bit
(sha256 c2b2217054f50429b644c7e66aab706147c64a20220372bb886946055d664529).

## Erratum 2 — 2026-09-29 post-collection audit (executed-code identity)

`last_commit_at_collection_start = 25849a7e…` names the last commit at
collection time, but the working tree then additionally held uncommitted
changes that are recorded verbatim as commit **6b227b7** (the `decompose`
subcommand, the `value_changed_nodes` effect field, and the validation
output-path fix — none of which existed at 25849a7, so 25849a7 alone could not
have produced the sealed artifacts). The executed code state therefore equals
6b227b7, verified by the bit-identical binary rebuild above; "clean of tracked
changes" held only after that commit was created. This file and SHA256SUMS
form the un-checksummed outer layer; the sealed JSONs named in SHA256SUMS are
unaffected by any edit to this file.

## Erratum 3 — 2026-09-29 closure pass (identity fields restructured)

The single `execution_revision` field was split into `executed_code_identity`
(immutable, above), `last_commit_at_collection_start`, and the separately
labeled documentation/audit heads, so a later documentation-only commit can
never be confused with the code state that produced the sealed evidence.
Executed-code identity is unchanged: **6b227b7**.
