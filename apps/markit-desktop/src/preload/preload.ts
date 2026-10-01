// Preload: the narrow bridge. Sandboxed (CJS output), contextIsolated.
// Only domain-shaped commands are exposed — never raw IPC, exec, fs, or a
// generic invoke(anyMethod, anyArgs).

const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("markitDesktop", {
  /**
   * Ask the user for a Markdown file. Resolves { path, source } or null.
   * FAKE-PATH: `source` rides along only until the Rust bridge exists.
   */
  openFile: (): Promise<{ path: string; source: string } | null> =>
    ipcRenderer.invoke("dialog:openMarkdown"),

  /**
   * Ask the user where to save and persist `bytesBase64`. Resolves the
   * chosen path or null. FAKE-PATH as above.
   */
  saveMarkdown: (
    suggestedName: string,
    bytesBase64: string,
  ): Promise<string | null> =>
    ipcRenderer.invoke("dialog:saveMarkdown", { suggestedName, bytesBase64 }),
});
