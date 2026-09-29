# HORSE-A-V2-DIAG-98 collection provenance
collected_utc = 2026-09-29T13:27:53Z
host = E5 (E5)
cpu = Intel(R) Xeon(R) CPU E5-2666 v3 @ 2.90GHz (20
scaling cores)
kernel = Linux 7.2.5-200.fc44.x86_64
os = Fedora release 44 (Forty Four)
rustc = rustc 1.98.1 (48a229cea 2026-09-01)
cargo = cargo 1.98.1 (797e8a9bc 2026-08-05)
llvm = LLVM version: 22.1.8
profile = release (opt-level 3, lto=thin, codegen-units 1, incremental=false, panic=unwind, target-cpu=default, empty RUSTFLAGS)
toolchain_binding = rust-toolchain.toml channel 1.97.1
execution_revision = 25849a7e923fb9219a227da3ff015347f6a35249
execution_branch = research/98-horse-a-v2-l0-l1-diagnosis
live_master_at_start = a89a4c2723390d903edb1b38543c263e0d042f05
baseline_capsule = markit-r0-rq8-research-record-v1.tar.gz
baseline_capsule_sha256 = 156ec1f3fbdfe1759ce81d17940360c2cda3adb773e7be7634b1ccc1e893d849
binary = target/release/mdbench-horse-a-v2-diag
commands =
  cargo build --release -p markit-mdbench-horse-a-v2-diag
  ./target/release/mdbench-horse-a-v2-diag all --out <dir>
  ./target/release/mdbench-horse-a-v2-diag decompose --out <dir>
  ./target/release/mdbench-horse-a-v2-diag manifest --out <dir>
