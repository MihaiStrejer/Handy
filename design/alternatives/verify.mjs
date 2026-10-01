import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const root = resolve("design/alternatives");
const targets = process.argv.slice(2);
const names = targets.length ? targets : ["a-flow-map", "b-storyboard", "c-lanes", "d-overview"];
await mkdir(`${root}/proof`, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  for (const name of names) {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    const external = [];
    page.on("pageerror", error => errors.push(error.message));
    page.on("request", request => { if (/^https?:/.test(request.url())) external.push(request.url()); });
    const target = name === "index" ? `${root}/index.html` : `${root}/${name}/index.html`;
    await page.goto(pathToFileURL(target).href);
    await page.waitForLoadState("load");
    await expect(page.locator("h1")).toBeVisible();
    const desktop = await page.evaluate(() => ({
      title: document.title,
      overflow: document.documentElement.scrollWidth > innerWidth,
      selects: document.querySelectorAll("select").length,
      brokenAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(link => link.hash.length > 1 && !document.getElementById(decodeURIComponent(link.hash.slice(1)))).map(link => link.hash),
      imagesLoaded: [...document.images].every(image => image.complete && image.naturalWidth > 0),
      headings: [...document.querySelectorAll("h2")].map(heading => heading.textContent.trim()),
    }));
    if (name === "index") {
      const localLinks = await page.locator("a[href]").evaluateAll(links => links.map(link => link.href).filter(href => href.startsWith("file:")));
      for (const href of localLinks) expect(existsSync(fileURLToPath(href)), `Missing local link: ${href}`).toBe(true);
    }
    expect(desktop.overflow, `${name}: desktop overflow`).toBe(false);
    expect(desktop.brokenAnchors, `${name}: broken section links`).toEqual([]);
    expect(desktop.imagesLoaded, `${name}: missing image`).toBe(true);
    await page.screenshot({ path: `${root}/proof/${name}-desktop.png` });
    const diagramSelector = { "a-flow-map": "#parallel", "b-storyboard": "#story", "c-lanes": "#sequence", "d-overview": "#overview" }[name];
    if (diagramSelector && await page.locator(diagramSelector).count()) {
      await page.locator(diagramSelector).evaluate(element => element.scrollIntoView({ block: "start", behavior: "instant" }));
    }
    await page.screenshot({ path: `${root}/proof/${name}-diagram.png` });
    await page.evaluate(() => window.scrollTo({ top: 0, behavior: "instant" }));
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: `${root}/proof/${name}-mobile.png` });
    const casesSelector = name === "a-flow-map" ? "#validation" : "#exceptions";
    if (await page.locator(casesSelector).count()) {
      await page.locator(casesSelector).evaluate(element => element.scrollIntoView({ block: "start", behavior: "instant" }));
      await page.screenshot({ path: `${root}/proof/${name}-mobile-cases.png` });
    }
    const mobileOverflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(mobileOverflow, `${name}: mobile overflow`).toBe(false);
    expect(errors, `${name}: browser errors`).toEqual([]);
    expect(external, `${name}: external assets`).toEqual([]);
    await page.close();
    const noScript = await browser.newPage({ javaScriptEnabled: false, viewport: { width: 1440, height: 1000 } });
    await noScript.goto(pathToFileURL(target).href);
    const text = await noScript.locator("body").innerText();
    const visibleCases = Object.fromEntries(Object.entries({ cancellation: /cancel/i, focus: /focus/i, field: /field|selection/i, invalidReply: /invalid|malformed/i, protectedInput: /protected/i }).map(([key, expression]) => [key, expression.test(text)]));
    if (name !== "index") {
      expect(Object.values(visibleCases).every(Boolean), `${name}: no-script cases`).toBe(true);
      expect(desktop.selects, `${name}: dropdowns obscure information`).toBe(0);
    }
    await noScript.close();
    const result = { checkedAt: new Date().toISOString(), browser: "Installed Microsoft Edge", desktop, mobileOverflow, errors, external, visibleCasesWithoutJavaScript: visibleCases };
    await writeFile(`${root}/proof/${name}.json`, JSON.stringify(result, null, 2) + "\n");
    console.log(`${name}: desktop/mobile/offline checks passed`);
  }
} finally {
  await browser.close();
}
