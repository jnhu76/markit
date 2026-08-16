// mvp/pocketjs/app/cf-notext.ts — A4-R1 counterfactual entry, no-text
// variant.
//
// DIAGNOSTIC ONLY: same imperative tree shape as cf.ts but the Text nodes
// stay empty — isolates text content work (JS slices + replaceText ops +
// core text layout) from tree/reconciliation work. Never a production
// implementation.

import { bootCf } from "./cf-boot.ts";

bootCf({ noText: true });
