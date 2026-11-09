import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

// The UI workspace this probe drives — resolved from THIS file (issue #36:
// was a hardcoded absolute worktree path), so the probe behaves identically
// no matter which worktree/checkout it runs from or what cwd npm gives it.
const UI = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const server = spawn("npm", ["run", "dev", "--", "--port", "5199", "--strictPort"], {
  cwd: UI, stdio: "ignore", detached: true,
});
await new Promise((r) => setTimeout(r, 5000));

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });

const routes = ["/player", "/playlist", "/settings/global", "/onboarding", "/media-info"];
const report = [];
for (const route of routes) {
  await page.goto(`http://localhost:5199${route}?locale=pseudo`, { waitUntil: "networkidle" });
  await page.waitForTimeout(700);
  const findings = await page.evaluate(() => {
    const doc = document.scrollingElement;
    const pageOverflowX = doc.scrollWidth - window.innerWidth;
    const clipped = [];
    for (const el of document.querySelectorAll("main *, [role] *")) {
      if (el.children.length === 0 && el.textContent && el.textContent.trim().length > 0) {
        const ellipsized = getComputedStyle(el).textOverflow === "ellipsis";
        if (ellipsized && el.scrollWidth > el.clientWidth + 1)
          clipped.push({ tag: el.tagName, cls: String(el.className).slice(0, 60), text: el.textContent.trim().slice(0, 30) });
      }
    }
    return { pageOverflowX, ellipsizedCount: clipped.length, samples: clipped.slice(0, 4), bracketed: document.body.textContent.includes("[") };
  });
  await page.screenshot({ path: `/home/z/pseudo-probe/pseudo-${route.replaceAll("/", "-") || "root"}.png` });
  report.push({ route, ...findings });
}
console.log(JSON.stringify(report, null, 2));
await browser.close();
process.kill(-server.pid, "SIGTERM");
