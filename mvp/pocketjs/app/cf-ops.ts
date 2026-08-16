// mvp/pocketjs/app/cf-ops.ts — op-count diagnostic entry for the A4-R1
// counterfactual (WRAP_OPS). Dedicated runs only.

import { perfSetWrapOps } from "./perf.ts";

perfSetWrapOps(true);

import { bootCf } from "./cf-boot.ts";

bootCf({ noText: false });
