// Native smoke (campaign §25): launch the built app under a virtual
// display, wait for the renderer-ready line from the main process, then
// exit. WSL2 note: needs a display server (xvfb-run works).
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";

const electron = "node_modules/.bin/electron";
if (!existsSync(electron)) {
  console.error("smoke: electron binary missing (run npm install)");
  process.exit(2);
}

const useXvfb = !process.env.DISPLAY;
const wrapper = useXvfb ? "xvfb-run" : null;
const args = useXvfb ? ["-a", electron, "--no-sandbox", "dist/main/main.js"] : [electron, "--no-sandbox", "dist/main/main.js"];

const child = spawn(wrapper ?? electron, args, { stdio: ["ignore", "pipe", "pipe"] });
let ready = false;
let mounted = false;
let output = "";
const timer = setTimeout(() => {
  console.error(`smoke: TIMEOUT — renderer-ready line not seen\n${output}`);
  child.kill("SIGKILL");
  process.exit(1);
}, 30_000);

child.stdout.on("data", (chunk) => {
  output += chunk.toString();
  if (output.includes("markit-desktop: renderer ready")) ready = true;
  if (output.includes("markit-desktop: workbench mounted=true")) mounted = true;
  if (ready && mounted) {
    clearTimeout(timer);
    child.kill("SIGTERM");
  }
});
child.stderr.on("data", (chunk) => {
  output += chunk.toString();
});
child.on("exit", (code) => {
  if (ready && mounted) {
    console.log("smoke: PASS (renderer ready, workbench mounted, clean shutdown)");
    process.exit(0);
  }
  console.error(
    `smoke: FAIL (exit ${code}, ready=${ready}, mounted=${mounted})\n${output}`,
  );
  process.exit(1);
});
