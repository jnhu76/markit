// V1 UI Slot registry — presentation placement only (architecture §11):
// typed slot ids, one declaration owner per slot, single|list kinds,
// stable contribution ids with deterministic ordering, idempotent
// withdrawal, owner withdrawal invalidating contributions, and stale
// render authorization rejected loudly. Deliberately NOT implemented:
// keyed dispatch, chains, shadow priority, factories, scopes, HMR.

/** A declared presentation placement. Ids are workbench vocabulary. */
export type SlotId =
  | "workbench.left-sidebar"
  | "editor.toolbar"
  | "workbench.status";

export type SlotKind = "single" | "list";

/** One presentation contribution. `render` mounts into the slot host. */
export interface SlotContribution {
  /** Stable within its slot; duplicates fail loudly. */
  readonly id: string;
  readonly render: (host: HTMLElement) => void;
}

/**
 * Live handle to one slot declaration — the render authorization. A
 * withdrawn declaration's handle is permanently stale: rendering or
 * contributing with it fails loudly, withdrawing through it is a no-op
 * (idempotent disposal).
 */
export interface SlotDeclaration {
  readonly slot: SlotId;
  readonly kind: SlotKind;
}

export class SlotRegistryError extends Error {}

interface SlotState {
  alive: boolean;
  contributions: Map<string, SlotContribution>;
}

export class SlotRegistry {
  readonly #slots = new Map<SlotId, SlotState>();
  /** Declaration-handle currency: a handle maps to the slot state it was
   * minted for; withdrawal severs the mapping. */
  readonly #handles = new WeakMap<SlotDeclaration, SlotState>();

  /**
   * Declare a slot placement. One declaration owner: declaring an
   * already-declared (live) slot fails loudly — conflict, never override.
   * Redeclaring a withdrawn slot is legal and yields a fresh handle.
   */
  declare(owner: string, slot: SlotId, kind: SlotKind): SlotDeclaration {
    const existing = this.#slots.get(slot);
    if (existing?.alive) {
      throw new SlotRegistryError(
        `slot ${slot} is already declared by its owner: slots have one declaration owner`,
      );
    }
    const state: SlotState = { alive: true, contributions: new Map() };
    this.#slots.set(slot, state);
    const declaration: SlotDeclaration = { slot, kind };
    this.#handles.set(declaration, state);
    void owner;
    return declaration;
  }

  /**
   * Contribute presentation to a live slot through a live declaration.
   * Returns the withdrawal function (idempotent). Duplicate contribution
   * ids and single-slot conflicts fail loudly; a stale declaration fails.
   */
  contribute(
    declaration: SlotDeclaration,
    contribution: SlotContribution,
  ): () => void {
    const state = this.#live(declaration);
    if (state.contributions.has(contribution.id)) {
      throw new SlotRegistryError(
        `contribution id ${contribution.id} already exists in ${declaration.slot}: ids are stable and unique`,
      );
    }
    if (declaration.kind === "single" && state.contributions.size >= 1) {
      throw new SlotRegistryError(
        `slot ${declaration.slot} is single and already has a contribution`,
      );
    }
    state.contributions.set(contribution.id, contribution);
    return () => this.withdrawContribution(declaration, contribution.id);
  }

  /** Withdraw one contribution. Idempotent; stale handles are no-ops. */
  withdrawContribution(declaration: SlotDeclaration, id: string): void {
    const state = this.#current(declaration);
    if (!state) return;
    state.contributions.delete(id);
  }

  /**
   * Withdraw the declaration. Removes the placement and invalidates every
   * contribution; the handle goes permanently stale. Idempotent.
   */
  withdrawDeclaration(declaration: SlotDeclaration): void {
    const state = this.#current(declaration);
    if (!state) return;
    state.alive = false;
    state.contributions.clear();
    this.#handles.delete(declaration);
  }

  /**
   * Render a slot into a host using a live declaration as the render
   * authorization. A stale authorization is rejected loudly — stale
   * render authority never draws.
   */
  render(declaration: SlotDeclaration, host: HTMLElement): void {
    const state = this.#live(declaration);
    host.replaceChildren();
    // Snapshot: a render callback that contributes mid-pass does not draw
    // in this pass (the next render picks it up).
    for (const contribution of [...state.contributions.values()]) {
      contribution.render(host);
    }
  }

  /** The state this handle currently names, if the handle is live. */
  #current(declaration: SlotDeclaration): SlotState | undefined {
    const state = this.#handles.get(declaration);
    return state !== undefined && this.#slots.get(declaration.slot) === state
      ? state
      : undefined;
  }

  #live(declaration: SlotDeclaration): SlotState {
    const state = this.#current(declaration);
    if (!state || !state.alive) {
      throw new SlotRegistryError(
        `stale authorization for slot ${declaration.slot}: the declaration was withdrawn or redeclared`,
      );
    }
    return state;
  }
}
