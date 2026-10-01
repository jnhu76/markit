// Renderer entry: composes the workbench shell with the domain adapter.
// FAKE-PATH (integration slice): the adapter here is the fake; switching
// to the real Rust bridge is a one-line swap plus deleting the fake.

import "./styles.css";
import { SlotRegistry } from "./slots/registry";
import { createWorkbench } from "./workbench/workbench";
import { OutlinePanel } from "./outline/outlinePanel";
import { createSourceEditor, type SourceEditor } from "./editor/sourceEditor";
import { StatusBar } from "./status/statusBar";
import { createFakeAdapter } from "./adapter/fake";
import type { DocumentSession } from "./adapter/types";

const adapter = createFakeAdapter();

const registry = new SlotRegistry();
const workbench = createWorkbench(registry);
document.body.replaceChildren(workbench.root);

const outline = new OutlinePanel(document.createElement("div"));
registry.contribute(workbench.leftSidebar, {
  id: outline.contributionId,
  render: (host) => host.append(outline.element()),
});

const statusBar = new StatusBar(document.createElement("div"));

let session: DocumentSession | null = null;
let editor: SourceEditor | null = null;
let filePath: string | null = null;

async function refresh(): Promise<void> {
  if (!session) return;
  const view = await session.view();
  const semantic = await session.semanticStatus();
  statusBar.update(
    view.revision,
    view.dirty ? "modified" : "saved",
    semantic.kind === "available"
      ? { kind: "ready", revision: semantic.revision }
      : semantic.kind === "failed"
        ? { kind: "failed", message: semantic.message }
        : { kind: "empty" },
  );
  if (semantic.kind === "failed") {
    outline.showUnavailable(semantic.message);
    return;
  }
  const pinned = await session.pin(view.revision);
  if (pinned) outline.show(pinned);
  workbench.renderSlots();
}

async function openDocument(source: string, path: string | null): Promise<void> {
  session = await adapter.openScratch(source);
  filePath = path;
  const host = workbench.root.querySelector<HTMLElement>('[data-slot-host="editor.surface"]');
  if (!host) throw new Error("missing editor surface");
  editor?.destroy();
  host.replaceChildren();
  editor = await createSourceEditor(host, session, () => void refresh());
  await refresh();
}

// Toolbar contributions: the early editor.toolbar list consumers.
function addToolbarButton(
  id: string,
  label: string,
  action: () => void,
): void {
  registry.contribute(workbench.toolbar, {
    id,
    render: (host) => {
      const button = document.createElement("button");
      button.textContent = label;
      button.addEventListener("click", action);
      host.append(button);
    },
  });
}

addToolbarButton("open", "Open", () => {
  void (async () => {
    const picked = await adapter.openFile();
    if (picked) await openDocument(picked.source, picked.path);
  })();
});

addToolbarButton("save", "Save", () => {
  void (async () => {
    if (!session) return;
    const view = await session.view();
    const result = await session.save(view.revision, filePath);
    if (result.kind === "saved") filePath = result.path;
    await refresh();
  })();
});

addToolbarButton("undo", "Undo", () => {
  void editor?.undo();
});

addToolbarButton("redo", "Redo", () => {
  void editor?.redo();
});

registry.contribute(workbench.status, {
  id: statusBar.contributionId,
  render: (host) => host.append(statusBar.element()),
});

workbench.renderSlots();

// The initial document is an unsaved scratch buffer (fake-path UI track).
void openDocument("# 欢迎使用 Markit\n\nEdit me — CJK · emoji 🌍 · Markdown.\n", null);
