import { chromium, expect } from "@playwright/test";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const root = resolve("design/alternatives/round-3");
const names = process.argv.slice(2);
if (!names.length) names.push("h-connected-map", "i-illustrated-session", "j-reader-questions");
await mkdir(`${root}/proof`, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  for (const name of names) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, javaScriptEnabled: false });
    const target = name === "index" ? `${root}/index.html` : `${root}/${name}/index.html`;
    const network = [];
    const errors = [];
    page.on("request", request => { if (/^https?:/.test(request.url())) network.push(request.url()); });
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(pathToFileURL(target).href);
    await expect(page.locator("h1")).toBeVisible();
    const result = await page.evaluate(() => ({
      title: document.title,
      desktopOverflow: document.documentElement.scrollWidth > innerWidth,
      missingAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(link => link.hash.length > 1 && !document.getElementById(decodeURIComponent(link.hash.slice(1)))).map(link => link.hash),
      imagesLoaded: [...document.images].every(image => image.complete && image.naturalWidth > 0),
      imageCount: document.images.length,
      selects: document.querySelectorAll("select").length,
      headings: [...document.querySelectorAll("h2")].map(heading => heading.textContent.trim()),
    }));
    expect(result.desktopOverflow).toBe(false);
    expect(result.missingAnchors).toEqual([]);
    expect(result.imagesLoaded).toBe(true);
    if (name !== "index") {
      const body = await page.locator("body").innerText();
      const patterns = { startup: /start|launch/i, parallel: /parallel|alongside|same time|independent/i, messages: /system/i, data: /transcript/i, cancel: /cancel/i, focus: /focus/i, field: /field|selection/i, invalid: /invalid|malformed/i, protected: /protected/i, proposed: /proposed|proposal|future/i, decisionA: /G1-A/, decisionB: /G1-B/, decisionC: /G1-C/ };
      result.coverage = Object.fromEntries(Object.entries(patterns).map(([key, pattern]) => [key, pattern.test(body)]));
      expect(result.imageCount).toBeGreaterThanOrEqual(2);
      expect(result.selects).toBe(0);
    }
    const links = await page.locator("a[href]").evaluateAll(links => links.map(link => link.href).filter(href => href.startsWith("file:")));
    result.missingLocalLinks = links.filter(href => !existsSync(fileURLToPath(href)));
    const pendingIndex = pathToFileURL(`${root}/index.html`).href;
    expect(result.missingLocalLinks.filter(href => name === "index" || href !== pendingIndex)).toEqual([]);
    await page.screenshot({ path: `${root}/proof/${name}-desktop.png` });
    const overview = page.locator('svg[role="img"], #map').first();
    if (await overview.count()) await overview.screenshot({ path: `${root}/proof/${name}-diagram.png` });
    const question = page.locator('#G1').first();
    if (await question.count()) await question.screenshot({ path: `${root}/proof/${name}-question.png` });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.evaluate(() => window.scrollTo({ top: 0, behavior: "instant" }));
    result.mobileOverflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(result.mobileOverflow).toBe(false);
    await page.screenshot({ path: `${root}/proof/${name}-mobile.png` });
    if (await question.count()) await question.screenshot({ path: `${root}/proof/${name}-question-mobile.png` });
    if (result.coverage) expect(Object.entries(result.coverage).filter(([, present]) => !present), "Missing visible review topics").toEqual([]);
    expect(network).toEqual([]);
    expect(errors).toEqual([]);
    Object.assign(result, { externalRequests: network, pageErrors: errors, javascript: "disabled", checkedAt: new Date().toISOString() });
    await writeFile(`${root}/proof/${name}.json`, JSON.stringify(result, null, 2) + "\n");
    console.log(`${name}: desktop/mobile, local links, images, no-script and no external requests passed`);
    await page.close();
  }
} finally {
  await browser.close();
}
