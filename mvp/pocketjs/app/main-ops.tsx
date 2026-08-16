// mvp/pocketjs/app/main-ops.tsx — op-count diagnostic entry (A4-R1).
//
// Same app as main.tsx with the native ui.* ops wrapped by counters
// (WRAP_OPS). Dedicated runs only — the wrap costs ~2 ms per edit turn,
// so it never ships in the timed battery.

import { perfSetWrapOps } from "./perf.ts";

perfSetWrapOps(true);

import Editor from "./app.tsx";
import { mount } from "@pocketjs/framework";
import { perfInit } from "./perf.ts";

perfInit();

mount(() => <Editor />);
