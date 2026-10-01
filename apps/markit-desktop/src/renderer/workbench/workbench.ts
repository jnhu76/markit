// The workbench shell: it OWNS the V1 slot declarations (left sidebar,
// editor toolbar, status) and composes the layout. It holds no document
// or semantic state — features contribute presentation to its slots and
// read the domain through the adapter.

import type { SlotDeclaration, SlotRegistry } from "../slots/registry";

export interface Workbench {
  readonly toolbar: SlotDeclaration;
  readonly leftSidebar: SlotDeclaration;
  readonly status: SlotDeclaration;
  readonly registry: SlotRegistry;
  readonly root: HTMLElement;
  renderSlots(): void;
}

export function createWorkbench(registry: SlotRegistry): Workbench {
  const root = document.createElement("div");
  root.className = "workbench";
  root.innerHTML = `
    <div class="workbench-topbar" data-slot-host="editor.toolbar"></div>
    <div class="workbench-main">
      <div class="workbench-sidebar" data-slot-host="workbench.left-sidebar"></div>
      <div class="workbench-editor" data-slot-host="editor.surface"></div>
    </div>
    <div class="workbench-status" data-slot-host="workbench.status"></div>
  `;

  const toolbar = registry.declare("workbench", "editor.toolbar", "list");
  const leftSidebar = registry.declare("workbench", "workbench.left-sidebar", "list");
  const status = registry.declare("workbench", "workbench.status", "list");

  return {
    toolbar,
    leftSidebar,
    status,
    registry,
    root,
    renderSlots() {
      registry.render(toolbar, host("editor.toolbar", root));
      registry.render(leftSidebar, host("workbench.left-sidebar", root));
      registry.render(status, host("workbench.status", root));
    },
  };
}

function host(name: string, root: HTMLElement): HTMLElement {
  const el = root.querySelector(`[data-slot-host="${name}"]`);
  if (!(el instanceof HTMLElement)) {
    throw new Error(`workbench is missing its own slot host: ${name}`);
  }
  return el;
}
