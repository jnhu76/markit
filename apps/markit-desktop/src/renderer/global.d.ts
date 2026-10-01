// The narrow preload surface exposed to the renderer (main/preload own
// the implementation; the renderer sees only this domain-shaped face).

export {};

declare global {
  interface Window {
    markitDesktop?: {
      openFile(): Promise<{ path: string; source: string } | null>;
      saveMarkdown(
        suggestedName: string,
        bytesBase64: string,
      ): Promise<string | null>;
    };
  }
}
