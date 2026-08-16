// mvp/pocketjs/app/md.tsx — A4-R2 Markdown L1 editor entry.
//
// Same host contract as main.tsx (mount + perfInit) with the L1 styled
// editor (md-app.tsx). Build: APP_ENTRY=md scripts/build-app.sh, run with
// --js markit-editor-md.js? No — the bundle is named md.js; pass
// --js/--pak explicitly (see bench/run-a4.py r2-* cells).

import MdEditor from "./md-app.tsx";
import { mount } from "@pocketjs/framework";
import { perfInit } from "./perf.ts";

perfInit();

mount(() => <MdEditor />);
