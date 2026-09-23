#!/usr/bin/env bash
# MARKIT-31 Stage-B perf-stat differential protocol (task §38).
# For each slot x horse:
#   rep1: --warmup 0 --iters 0   (frozen-workload load baseline only)
#   rep2/3: --warmup 10 --iters 20000 (edit loop dominates)
# per-edit cycles = (C_N - C_0) / N. Repetitions are NOT primary samples.
set -u
PROF=analysis/primary/905427da/profiling
REPLAY=$PWD/analysis/scripts/profiling-replay/target/release/profiling-replay
ROOT=$PWD
EVENTS=task-clock,cycles,instructions,branches,branch-misses,cache-references,cache-misses,page-faults,context-switches,cpu-migrations

run() { # name case horse mode outtag iters warmup
  local name=$1 case=$2 horse=$3 iters=$4 warmup=$5 tag=$6
  perf stat -x, -e $EVENTS -o $PROF/perf-stat/$name-$horse-$tag.csv -- \
    taskset -c 1 $REPLAY --benchmark-root $ROOT --case $case --horse $horse \
    --warmup $warmup --iters $iters > /dev/null 2>&1
}

slot() { # name case horse
  local name=$1 case=$2 horse=$3
  run $name $case $horse 0 0 base
  run $name $case $horse 20000 10 loop
  echo "$name $horse ok"
}

slot P1 9946b5d49a56529eebb9de39e11cf1b90fdce580569534ab54d782c00ba8b593 H0
slot P1 9946b5d49a56529eebb9de39e11cf1b90fdce580569534ab54d782c00ba8b593 H1
slot P2 a3d4da7ba3d5b5dc18b530f3f465498db4aa464e7087e5dce79a0d4828fc1f3f H0
slot P2 a3d4da7ba3d5b5dc18b530f3f465498db4aa464e7087e5dce79a0d4828fc1f3f H1
slot P3 abf0d9be96b4f1ee1cc656f1d4760bb7b208201baef57c703ddaf7ddf84c6fb2 H3
slot P4 fea305d72dbbeee48fdea65ea9893a4889602407d2c96d9baf476141ba785ac3 H3
slot P4 fea305d72dbbeee48fdea65ea9893a4889602407d2c96d9baf476141ba785ac3 H2
slot P5 1a0deadd0f109c2a22aacdb0046eabc2a7a4ee1666b254455b1a07e548e44369 H2
slot P6 73039631b4e45389797f5ef0bb26cc57ac1f3da406ae556a70236a194049d66a H2
slot P7 73039631b4e45389797f5ef0bb26cc57ac1f3da406ae556a70236a194049d66a H4
slot P8 d0a2203aa2451c26f9c1f910511bd35f908ddec5cd61bcfd575d67400cfec5f5 H4
slot P9 d5502cc52c2173802f4aa342217f7333ea54ca238086c96a253ff4568a0a2828 H4
slot P9 d5502cc52c2173802f4aa342217f7333ea54ca238086c96a253ff4568a0a2828 H0
echo ALL-SLOTS-DONE
