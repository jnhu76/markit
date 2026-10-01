// The product domain face the renderer consumes. These types mirror the
// FROZEN product semantics only (document identity, revision, source,
// headings, dirty, edit, undo, redo, save, semantic availability) — the
// same semantics `markit-document` and `markit-markdown-api` define on the
// Rust side. The renderer never invents new domain authority here.

/**
 * A UTF-8 byte offset into one revision of a document's source. The
 * adapter converts from UI-native coordinates at this boundary.
 */
export type SourceOffset = number;

/** Half-open byte range [start, end) in one revision's source. */
export interface SourceRange {
  start: SourceOffset;
  end: SourceOffset;
}

export interface DocumentView {
  documentId: number;
  revision: number;
  source: string;
  dirty: boolean;
}

export interface HeadingView {
  level: number;
  text: string;
  range: SourceRange;
}

/** Post-fact semantic availability (frozen push face). */
export type SemanticStatusView =
  | { kind: "empty" }
  | { kind: "available"; revision: number }
  | { kind: "failed"; revision: number; message: string };

export interface OutlineView {
  revision: number;
  entries: HeadingView[];
}

/**
 * Why a domain command was refused. Refusals never mutate the document:
 * the renderer resyncs from the authoritative snapshot instead.
 */
export type DomainErrorView =
  | { kind: "staleBase"; currentRevision: number }
  | { kind: "invalidRange" }
  | { kind: "notCharBoundary" }
  | { kind: "documentMismatch" }
  | { kind: "saveFailed"; message: string };

export type EditResultView =
  | { kind: "committed"; revision: number }
  | { kind: "rejected"; error: DomainErrorView };

export interface DocumentHandle {
  readonly documentId: number;
}

/**
 * One open document's domain face. Every mutating command carries the
 * caller's base revision; stale commands are rejected explicitly and the
 * renderer resyncs. All methods are async so the fake adapter (UI track)
 * and the real bridge (integration slice) are interchangeable.
 */
export interface DocumentSession {
  readonly handle: DocumentHandle;

  /** The authoritative current state (source + revision + dirty). */
  view(): Promise<DocumentView>;

  /** Commit one byte-range replacement against `baseRevision`. */
  edit(
    baseRevision: number,
    range: SourceRange,
    replacement: string,
  ): Promise<EditResultView>;

  /** Undo is a new commit; revision advances, content returns. */
  undo(): Promise<number | null>;

  /** Redo replays the most recently undone commit as a new one. */
  redo(): Promise<number | null>;

  /** The pinned view for a revision (never upgrades to latest). */
  pin(revision: number): Promise<OutlineView | null>;

  /** Persist one revision's bytes; marks exactly that revision saved. */
  save(revision: number, path: string | null): Promise<
    | { kind: "saved"; savedRevision: number; path: string }
    | { kind: "rejected"; error: DomainErrorView }
  >;

  /** Latest observable semantic availability for this document. */
  semanticStatus(): Promise<SemanticStatusView>;
}

/** The composition-level face: open documents from files or scratch. */
export interface DomainAdapter {
  openFile(): Promise<{ path: string; source: string } | null>;
  openScratch(source: string): Promise<DocumentSession>;
}
