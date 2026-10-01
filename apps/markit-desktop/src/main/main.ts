// Electron main process: window lifecycle plus the two narrow dialog
// commands the renderer is allowed to ask for. The renderer has no Node
// authority (contextIsolation, sandbox, no nodeIntegration) and sees only
// the domain-shaped faces the preload exposes.
//
// FAKE-PATH (integration slice removes this): open/save handlers read and
// write file bytes here so the UI track can run before the Rust bridge
// exists. When the real bridge lands, these handlers shrink to returning
// the picked PATH and all byte movement moves behind the domain bridge.

import { app, BrowserWindow, dialog, ipcMain } from "electron";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

// ESM main: derive the bundle directory from the module URL.
const dirname = path.dirname(fileURLToPath(import.meta.url));

function createWindow(): BrowserWindow {
  const win = new BrowserWindow({
    width: 1100,
    height: 760,
    title: "Markit",
    show: false,
    webPreferences: {
      preload: path.join(dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      webSecurity: true,
    },
  });

  // No window.open authorities: the shell denies all new windows.
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  win.once("ready-to-show", () => {
    win.show();
    // The native smoke observes this line; keep the wording stable.
    console.log("markit-desktop: renderer ready");
  });

  const index = path.join(dirname, "../renderer/index.html");
  void win.loadFile(index);
  return win;
}

ipcMain.handle("dialog:openMarkdown", async () => {
  const win = BrowserWindow.getAllWindows().at(-1);
  if (!win) return null;
  const picked = await dialog.showOpenDialog(win, {
    properties: ["openFile"],
    filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
  });
  if (picked.canceled || picked.filePaths.length === 0) return null;
  const file = picked.filePaths[0]!;
  // FAKE-PATH: byte movement moves to the Rust bridge in the integration
  // slice; until then the main process reads the picked file.
  const source = fs.readFileSync(file, "utf8");
  return { path: file, source };
});

ipcMain.handle(
  "dialog:saveMarkdown",
  async (_event, args: { suggestedName: string; bytesBase64: string }) => {
    const win = BrowserWindow.getAllWindows().at(-1);
    if (!win) return null;
    const picked = await dialog.showSaveDialog(win, {
      defaultPath: args.suggestedName,
      filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
    });
    if (picked.canceled || !picked.filePath) return null;
    fs.writeFileSync(picked.filePath, Buffer.from(args.bytesBase64, "base64"));
    return picked.filePath;
  },
);

app.whenReady().then(() => {
  createWindow();
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("window-all-closed", () => {
  app.quit();
});
