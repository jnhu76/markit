# HORSE-A-V2-L2-100 collection provenance

collected_utc = 2026-09-29T23:59Z .. 2026-09-30T00:20Z (pilot + campaign)
host = jnhu@192.168.31.75 (hostname E5)
cpu = Intel(R) Xeon(R) Core E5-2666 v3 @ 2.90GHz (20 cores, Haswell-EP)
kernel = Linux 7.2.5-200.fc44.x86_64 (Fedora 44)
perf = perf version 7.2.5-200.fc44.x86_64
governor = schedutil (untouched); every perf run pinned with `taskset -c 2`

# --- authority ---

issue = #100 (live body fetched and followed; live == execution prompt contract)
parent_authority = #95 direction / #96 methodology / #97 umbrella /
                   #98 CLOSED L0/L1 (results/horse-a-v2-diag-98/, untouched)
live_master_at_start = 6d88becbb94993618321bd71803ce95bdc09e454
                     (matched the expected SHA in the #100 execution contract;
                      no intervening commits)
execution_branch = research/100-horse-a-v2-l2-context-profile

# --- toolchain (workspace pin) ---

rustc = rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo = cargo 1.97.1 (c980f4866 2026-06-30)
llvm = 22.1.6 (bundled with the 1.97.1 pin; rust-toolchain.toml)

# --- profiling build identity (P-LANE ONLY; not a T-LANE profile) ---

build_command = RUSTFLAGS="-C force-frame-pointers=yes" \
                cargo build --profile release-l2-profile-v1 \
                  -p markit-mdbench-horse-a-v2-l2
profile = release-l2-profile-v1 (inherits the frozen [profile.release]:
          opt-level 3, lto=thin, codegen-units 1, incremental=false,
          panic=unwind, default target-cpu; adds debug = 2 ONLY)
rustflags = "-C force-frame-pointers=yes" (recorded; the frame-pointer lane)
binary = target/release-l2-profile-v1/mdbench-horse-a-v2-l2
binary_sha256 = ee9b680085128993507fecbeb9fbbb141d0358490a455af56d051de63592de09
pair_identity = ONE binary family for P1 and P0, all cells, both repeats.
P-LANE RULE = wall times of this build are NOT latency evidence; the #98
          T-LANE remains the latency authority.

# --- substrate decisions recorded for review ---

1. First attempt: the existing `release` binary with
   `--call-graph dwarf,16384`. Without debug info, thin-LTO-inlined parser
   frames are invisible (E6-1/P1 pilot: only 14 `full_build` frames vs 425
   entry frames), and with a debug=2 build the 16 KiB DWARF stack dumps
   TRUNCATED deep parser chains arm-asymmetrically (E6-1/P0 pilot: only
   188/3852 samples retained the entry frame). PILOT REJECT.
2. Fix (this campaign): `release-l2-profile-v1` = release + debug=2 +
   force-frame-pointers. FP chains do not truncate; `perf script --inline`
   expands inlined frames from the DWARF info. E6-1 pilots after the fix:
   P1 5995/6194 in-region (96.8%), resolved 99.0%; P0 3741/3896 (96.0%),
   resolved 96.2%. PILOT PASS -> contract frozen -> campaign collected.
3. Host tweak: kernel.perf_event_mlock_kb 516 -> 8192 (sudo; ring capacity
   so DWARF/FP samples are not dropped under IO load; P-LANE infrastructure
   only, reversible, does not touch measurement semantics).

# --- exact execution commands ---

build = see build_command above
collect (per profile, CELL/ARM/REPS/WARMUP per MANIFEST.md):
  taskset -c 2 perf record -e cycles:u -c 100000 --call-graph fp \
    -o <raw/perf_CELL_ARM_REP.data> -- \
    target/release-l2-profile-v1/mdbench-horse-a-v2-l2 \
    --cell CELL --arm ARM --reps REPS --warmup WARMUP
  perf script --inline -i <raw data> > <mid txt>
  python3 horse-a-v2-l2/l2tools/perf_fold.py --in <mid> --out <folded> --quality <q>
attribution:
  python3 horse-a-v2-l2/l2tools/classify.py --folded <folded> \
    --cell CELL --arm ARM --reps REPS --out <attribution json>
differential:
  python3 horse-a-v2-l2/l2tools/differential.py \
    --evidence results/horse-a-v2-l2-100/campaign/receipts \
    --cells E6-1 E6-5 E6-6 --tag rep1|rep2 \
    --out-json <diff json> --out-md <diff md>

The attribution REGENERATES from the retained raw perf.data by the exact
commands above (validation requirement of #100).

# --- raw data policy ---

raw perf.data and mid perf-script dumps stay on the execution host under
results/horse-a-v2-l2-100/campaign/{raw,mid}/ (git-ignored). Their SHA-256
sums are in receipts/sha256_*.txt. The committed review evidence is the
folded stacks, quality metrics, attribution JSONs, differential tables,
harness receipts, and this provenance; folded data regenerates the full
attribution, and raw data regenerates the folded data.
