import { chromium, expect } from "@playwright/test";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const root = resolve("design/alternatives/round-2");
const names = process.argv.slice(2);
if (!names.length) names.push("e-purpose-lanes", "f-two-runs", "g-boundaries");
await mkdir(`${root}/proof`, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  for (const name of names) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, javaScriptEnabled: false });
    const target = name === "index" ? `${root}/index.html` : `${root}/${name}/index.html`;
    const network = [];
    page.on("request", request => { if (/^https?:/.test(request.url())) network.push(request.url()); });
    await page.goto(pathToFileURL(target).href);
    await expect(page.locator("h1")).toBeVisible();
    const result = await page.evaluate(() => ({
      title: document.title,
      desktopOverflow: document.documentElement.scrollWidth > innerWidth,
      missingAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(link => link.hash.length > 1 && !document.getElementById(decodeURIComponent(link.hash.slice(1)))).map(link => link.hash),
      imagesLoaded: [...document.images].every(image => image.complete && image.naturalWidth > 0),
      selects: document.querySelectorAll("select").length,
      headings: [...document.querySelectorAll("h2")].map(heading => heading.textContent.trim()),
    }));
    expect(result.desktopOverflow).toBe(false);
    expect(result.missingAnchors).toEqual([]);
    expect(result.imagesLoaded).toBe(true);
    if (name !== "index") {
      const body = await page.locator("body").innerText();
      result.caseCoverage = Object.fromEntries(Object.entries({ cancel: /cancel/i, focus: /focus/i, changedField: /field|selection/i, invalidReply: /invalid|malformed/i, protected: /protected/i, proposed: /proposed|proposal/i }).map(([key, pattern]) => [key, pattern.test(body)]));
      expect(Object.values(result.caseCoverage).every(Boolean)).toBe(true);
      expect(result.selects).toBe(0);
    }
    const links = await page.locator("a[href]").evaluateAll(links => links.map(link => link.href).filter(href => href.startsWith("file:")));
    result.missingLocalLinks = links.filter(href => !existsSync(fileURLToPath(href)));
    // The comparison index is created after the individual page previews.
    const pendingIndex = pathToFileURL(`${root}/index.html`).href;
    expect(result.missingLocalLinks.filter(href => name === "index" || href !== pendingIndex)).toEqual([]);
    await page.screenshot({ path: `${root}/proof/${name}-desktop.png` });
    if (name === "index") await page.locator('#retained').screenshot({ path: `${root}/proof/index-retained-desktop.png` });
    await page.setViewportSize({ width: 390, height: 844 });
    result.mobileOverflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(result.mobileOverflow).toBe(false);
    await page.screenshot({ path: `${root}/proof/${name}-mobile.png` });
    if (name === "index") await page.locator('#retained').screenshot({ path: `${root}/proof/index-retained-mobile.png` });
    const exceptions = page.locator('#exceptions, #failures, [aria-label="Exceptions"]').first();
    if (await exceptions.count()) {
      await exceptions.evaluate(element => element.scrollIntoView({ block: "start", behavior: "instant" }));
      await page.screenshot({ path: `${root}/proof/${name}-mobile-exceptions.png` });
    }
    expect(network).toEqual([]);
    result.externalRequests = network;
    result.javascript = "disabled; core reading inspected without scripts";
    result.checkedAt = new Date().toISOString();
    await writeFile(`${root}/proof/${name}.json`, JSON.stringify(result, null, 2) + "\n");
    console.log(`${name}: desktop/mobile, offline and no-script checks passed`);
    await page.close();
  }
} finally {
  await browser.close();
}
