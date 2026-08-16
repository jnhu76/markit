// mvp/pocketjs/app/platform.ts — A4-P platform adapter contracts.
//
// The capability-driven interfaces the product core codes against
// (ADR-002). NO implementations here: each platform (windows / linux /
// macos) provides one, wired through the host/svc boundary or a direct
// host op. The core never branches on the platform — it consumes these
// contracts.
//
// P1 implements: Windows ClipboardProvider, FontProvider, ImeProvider,
// FileDialogProvider (see docs/product/issue-backlog.md).

/** Command the editor core emits; platforms bind shortcuts (ShortcutPolicy). */
export type Command =
  | "copy" | "cut" | "paste"
  | "undo" | "redo"
  | "select-all"
  | "save" | "save-as" | "open" | "find";

/** Text clipboard: text-only for the MVP (rich/html/images deferred). */
export interface ClipboardProvider {
  readText(): string;
  writeText(text: string): void;
}

/** A resolved font face for a text run. */
export interface FontFace {
  /** Platform font name (GDI / fontconfig / CoreText). */
  name: string;
  /** Path the PocketJS runtime-glyph path can load, when applicable. */
  path?: string;
}

/**
 * System font discovery + fallback chain (Latin → CJK → emoji).
 * Never a hardcoded OS font path list — discovery is per platform and
 * the chain is data-driven.
 */
export interface FontProvider {
  /** The fallback chain for a text sample, best first. */
  fallbackChain(sample: string): FontFace[];
  /** Resolve one face for a slot (weight/size), for the baked atlases. */
  resolveSlot(slot: number): FontFace;
}

/** IME composition — the editor model's four states (ADR-007). */
export interface ImeProvider {
  /** The host reported composition start at a caret offset. */
  onCompositionStart(offset: number): void;
  /** Preedit text update (never enters the undo stack). */
  onCompositionUpdate(text: string, caretInText: number): void;
  /** Commit: one edit transaction (grouped for undo). */
  onCompositionCommit(text: string): void;
  onCompositionCancel(): void;
}

/** Native open/save dialogs (MVP: text/UTF-8 files). */
export interface FileDialogProvider {
  open(options: { filter?: string }): Promise<string | null>;
  save(options: { defaultName?: string }): Promise<string | null>;
}

/**
 * Command → platform binding (Ctrl on Windows/Linux, Cmd on macOS).
 * The core emits Command values; the platform maps them to chords.
 */
export interface ShortcutPolicy {
  chordFor(command: Command): string;
}

/** Standard per-platform directories (never hardcode ~/.markit). */
export interface PlatformPaths {
  configDir(): string;
  cacheDir(): string;
  recoveryDir(): string;
  logsDir(): string;
  recentFilesPath(): string;
}
