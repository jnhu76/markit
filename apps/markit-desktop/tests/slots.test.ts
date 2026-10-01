// V1 Slot registry semantics tests (architecture §11 frozen subset).

import { describe, expect, it } from "vitest";
import { SlotRegistry, SlotRegistryError, type SlotDeclaration } from "../src/renderer/slots/registry";

function mount(): { text: string } {
  return { text: "mounted" };
}

describe("SlotRegistry", () => {
  it("one declaration owner: a live second declaration fails loudly", () => {
    const registry = new SlotRegistry();
    registry.declare("workbench", "editor.toolbar", "list");
    expect(() => registry.declare("other", "editor.toolbar", "list")).toThrow(
      SlotRegistryError,
    );
  });

  it("list contributions render in explicit registration order with stable ids", () => {
    const registry = new SlotRegistry();
    const toolbar = registry.declare("workbench", "editor.toolbar", "list");
    const host = document.createElement("div");
    registry.contribute(toolbar, {
      id: "open",
      render: (h) => {
        const b = document.createElement("button");
        b.textContent = "open";
        h.append(b);
      },
    });
    registry.contribute(toolbar, {
      id: "save",
      render: (h) => {
        const b = document.createElement("button");
        b.textContent = "save";
        h.append(b);
      },
    });
    registry.render(toolbar, host);
    expect([...host.querySelectorAll("button")].map((b) => b.textContent)).toEqual([
      "open",
      "save",
    ]);
  });

  it("duplicate contribution ids fail loudly", () => {
    const registry = new SlotRegistry();
    const toolbar = registry.declare("workbench", "editor.toolbar", "list");
    const c = { id: "open", render: () => {} };
    registry.contribute(toolbar, c);
    expect(() => registry.contribute(toolbar, { ...c })).toThrow(SlotRegistryError);
  });

  it("single slots reject a second contribution", () => {
    const registry = new SlotRegistry();
    const slot = registry.declare("workbench", "workbench.status", "single");
    registry.contribute(slot, { id: "a", render: () => {} });
    expect(() => registry.contribute(slot, { id: "b", render: () => {} })).toThrow(
      SlotRegistryError,
    );
  });

  it("contribution withdrawal is idempotent", () => {
    const registry = new SlotRegistry();
    const toolbar = registry.declare("workbench", "editor.toolbar", "list");
    const withdraw = registry.contribute(toolbar, {
      id: "open",
      render: () => {},
    });
    withdraw();
    expect(() => withdraw()).not.toThrow();
    const host = document.createElement("div");
    registry.render(toolbar, host);
    expect(host.children).toHaveLength(0);
  });

  it("owner withdrawal invalidates contributions; later contribute and render fail", () => {
    const registry = new SlotRegistry();
    const toolbar = registry.declare("workbench", "editor.toolbar", "list");
    registry.contribute(toolbar, { id: "open", render: () => {} });
    registry.withdrawDeclaration(toolbar);
    expect(() => registry.contribute(toolbar, { id: "save", render: () => {} })).toThrow(
      SlotRegistryError,
    );
    expect(() => registry.render(toolbar, document.createElement("div"))).toThrow(
      SlotRegistryError,
    );
  });

  it("withdrawal is idempotent for declarations too", () => {
    const registry = new SlotRegistry();
    const toolbar = registry.declare("workbench", "editor.toolbar", "list");
    registry.withdrawDeclaration(toolbar);
    expect(() => registry.withdrawDeclaration(toolbar)).not.toThrow();
  });

  it("redeclaring a withdrawn slot yields a fresh authority; the old handle stays stale", () => {
    const registry = new SlotRegistry();
    const first: SlotDeclaration = registry.declare("a", "editor.toolbar", "list");
    registry.withdrawDeclaration(first);
    const second = registry.declare("b", "editor.toolbar", "list");
    expect(() => registry.render(first, document.createElement("div"))).toThrow(
      SlotRegistryError,
    );
    registry.contribute(second, { id: "open", render: mount as never });
    const host = document.createElement("div");
    expect(() => registry.render(second, host)).not.toThrow();
  });
});
