// @title Markit PocketJS Thin Editor
//
// The thin-editor guest: a plain-text editing surface over the Markit
// flat widget host (mvp/pocketjs/src/main.rs). Build with:
//
//   bun tools/build.ts markit-editor --density=1 --font-regular=<Consolas>
//     (see mvp/pocketjs/scripts/build-app.sh)
//
// No markdown: the document is raw text, byte for byte. The host feeds
// keyboard/mouse/scroll/resize through the svc channel; without a host
// the bundle renders the seed document read-only.

import Editor from "./app.tsx";
import { mount } from "@pocketjs/framework";
import { perfInit } from "./perf.ts";

// Phase A2: wrap the native ui ops with counters before the first render.
perfInit();

mount(() => <Editor />);
