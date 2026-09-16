// Lezer Markdown persistent measurement worker (issue #19 RUN-2c,
// plan §11 timing rule: the parser stays warm, JS measures its own
// native parse time, and the harness reports transport separately).
//
// Protocol: JSON lines on stdin → JSON lines on stdout.
//   {"cmd":"full","doc":...,"iters":N}
//     → {"ok":true,"native_us":median,"node_count":n,"tree_len":n}
//   {"cmd":"inc","old_doc":...,"new_doc":...,"fromA":n,"toA":n,"fromB":n,"toB":n,"iters":N}
//     → {"ok":true,"native_us":median,"adjust_us":us,"node_count":n,"tree_len":n}
//   {"cmd":"stop"}
//
// Reuse evidence: Lezer exposes no changed-range/node-reuse counters,
// so the honest proxies are (a) inc-vs-full native time on the same
// document and (b) the observable fragment flow (addTree → applyChanges
// → parse(new, fragments)). Node counts are measured by walking the
// tree OUTSIDE the timed region.

const { parser } = require("@lezer/markdown");
const { TreeFragment } = require("@lezer/common");
const readline = require("readline");

function median(arr) {
  arr.sort((a, b) => a - b);
  return arr[Math.floor(arr.length / 2)];
}

function usSince(t0) {
  return Number(process.hrtime.bigint() - t0) / 1000;
}

function countNodes(tree) {
  let n = 0;
  tree.iterate({
    from: 0,
    to: tree.length,
    enter() {
      n++;
      return true;
    },
    leave() {},
  });
  return n;
}

function gc() {
  if (typeof global.gc === "function") global.gc();
}

const rl = readline.createInterface({ input: process.stdin });
rl.on("line", (line) => {
  if (!line.trim()) return;
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (e) {
    process.stdout.write(JSON.stringify({ ok: false, error: "bad json" }) + "\n");
    return;
  }
  try {
    if (msg.cmd === "stop") {
      process.exit(0);
    } else if (msg.cmd === "full") {
      gc();
      // warm-up
      parser.parse(msg.doc);
      const times = [];
      let lastTree = null;
      for (let i = 0; i < msg.iters; i++) {
        const t0 = process.hrtime.bigint();
        lastTree = parser.parse(msg.doc);
        times.push(usSince(t0));
      }
      process.stdout.write(
        JSON.stringify({
          ok: true,
          native_us: median(times),
          node_count: countNodes(lastTree),
          tree_len: lastTree.length,
        }) + "\n"
      );
    } else if (msg.cmd === "inc") {
      gc();
      const oldTree = parser.parse(msg.old_doc);
      const times = [];
      let lastTree = null;
      let adjustUs = 0;
      const changes = [
        { fromA: msg.fromA, toA: msg.toA, fromB: msg.fromB, toB: msg.toB },
      ];
      for (let i = 0; i < msg.iters; i++) {
        // Fragment adjustment is the representation-maintenance step —
        // timed separately from the parse (plan §11 record split).
        const tA = process.hrtime.bigint();
        const frags = TreeFragment.applyChanges(
          TreeFragment.addTree(oldTree),
          changes
        );
        const tAdjust = usSince(tA);
        if (i > 0) adjustUs += tAdjust;
        const t0 = process.hrtime.bigint();
        lastTree = parser.parse(msg.new_doc, frags);
        times.push(usSince(t0));
      }
      process.stdout.write(
        JSON.stringify({
          ok: true,
          native_us: median(times),
          adjust_us: adjustUs / Math.max(1, msg.iters - 1),
          node_count: countNodes(lastTree),
          tree_len: lastTree.length,
        }) + "\n"
      );
    } else {
      process.stdout.write(JSON.stringify({ ok: false, error: "unknown cmd" }) + "\n");
    }
  } catch (e) {
    process.stdout.write(JSON.stringify({ ok: false, error: String(e) }) + "\n");
  }
});
