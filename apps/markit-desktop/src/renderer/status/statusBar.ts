// Status chips: revision, saved/dirty, semantic availability. Presented
// through the workbench.status list slot; every chip is presentation of
// domain facts it is handed, never a state owner.

export type SaveState = "saved" | "modified" | "unsaved";
export type SemanticState =
  | { kind: "empty" }
  | { kind: "ready"; revision: number }
  | { kind: "failed"; message: string };

export class StatusBar {
  readonly #host: HTMLElement;
  #revision = 0;
  #save: SaveState = "unsaved";
  #semantic: SemanticState = { kind: "empty" };

  constructor(host: HTMLElement) {
    this.#host = host;
    this.#host.className = "status-bar";
  }

  get contributionId(): string {
    return "status-chips";
  }

  update(revision: number, save: SaveState, semantic: SemanticState): void {
    this.#revision = revision;
    this.#save = save;
    this.#semantic = semantic;
    this.#render();
  }

  render(host: HTMLElement): void {
    void host; // the chip renders into its own element, contributed below
  }

  element(): HTMLElement {
    return this.#host;
  }

  #render(): void {
    this.#host.replaceChildren();
    const chip = (text: string, cls: string) => {
      const el = document.createElement("span");
      el.className = `status-chip ${cls}`;
      el.textContent = text;
      this.#host.append(el);
    };
    chip(`Revision ${this.#revision}`, "status-revision");
    chip(
      this.#save === "saved" ? "Saved" : this.#save === "modified" ? "Modified" : "Unsaved",
      `status-save-${this.#save}`,
    );
    switch (this.#semantic.kind) {
      case "ready":
        chip("Semantic Ready", "status-semantic-ready");
        break;
      case "failed":
        chip(`Semantic Failed — ${this.#semantic.message}`, "status-semantic-failed");
        break;
      case "empty":
        chip("Semantic …", "status-semantic-empty");
        break;
    }
  }
}
