// The Outline panel: a consumer of the pinned semantic view. It renders
// whatever the current OutlineView holds and never reinterprets Markdown
// itself — headings come from the domain face (the Markdown capability on
// the Rust side, the fake scan here).

import type { OutlineView } from "../adapter/types";

export class OutlinePanel {
  readonly #host: HTMLElement;
  #current: OutlineView | null = null;

  constructor(host: HTMLElement) {
    this.#host = host;
    this.#host.className = "outline-panel";
  }

  get contributionId(): string {
    return "outline";
  }

  element(): HTMLElement {
    return this.#host;
  }

  /** Coarse full refresh per semantic revision (architecture §5.3). */
  show(outline: OutlineView): void {
    this.#current = outline;
    this.#render();
  }

  /** A provider failure is shown as a failure — the old outline is never
   * relabeled as current. */
  showUnavailable(message: string): void {
    this.#current = null;
    this.#host.replaceChildren();
    const label = document.createElement("div");
    label.className = "outline-unavailable";
    label.textContent = `Outline unavailable: ${message}`;
    this.#host.append(label);
  }

  #render(): void {
    this.#host.replaceChildren();
    const outline = this.#current;
    if (!outline) return;
    const title = document.createElement("div");
    title.className = "outline-title";
    title.textContent = "Outline";
    this.#host.append(title);
    for (const entry of outline.entries) {
      const row = document.createElement("div");
      row.className = "outline-row";
      row.style.paddingLeft = `${(entry.level - 1) * 14}px`;
      row.textContent = entry.text || "(untitled)";
      this.#host.append(row);
    }
  }
}
