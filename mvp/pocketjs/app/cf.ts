// mvp/pocketjs/app/cf.ts — A4-R1 counterfactual entry (text path).
//
// DIAGNOSTIC ONLY (Phase A4-R1 §6): the same editor model and the same
// visible presentation as app.tsx, driven without Solid reactivity. Never
// a production implementation. Build: APP_ENTRY=cf scripts/build-app.sh

import { bootCf } from "./cf-boot.ts";

bootCf({ noText: false });
