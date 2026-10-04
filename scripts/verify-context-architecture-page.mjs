// Historical schema-1 page verifier; its payload is not the current API.
import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const proof = "design/proof/architecture";
await mkdir(proof, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const network = [];
page.on("request", (request) => {
  if (/^https?:/.test(request.url())) network.push(request.url());
});
try {
  await page.goto(
    pathToFileURL(resolve("design/context-profiles-architecture.html")).href,
  );
  await expect(page.locator("h1")).toContainText("A voice session");
  await expect(page.locator("#stage h3")).toContainText("Load services");
  await page.locator("#next").click();
  await expect(page.locator("#counter")).toHaveText("2 / 10");
  await page.locator("#scenario").selectOption("edit");
  await expect(page.locator("#reply")).toContainText("replace_selection");
  await expect(page.locator("#payload")).toContainText("cloud code");
  await page.locator(".steps button").nth(8).click();
  await expect(page.locator("#stage h3")).toContainText("existing output path");
  await page.locator("#scenario").selectOption("general");
  await expect(page.locator("#reply")).toContainText('"operation": "insert"');
  await expect(page.locator("#payload")).toContainText('"dictionary": []');
  await page.locator("#failure").selectOption("invalid");
  await expect(page.locator("#failure-result")).toContainText("malformed JSON");
  await page.locator('a[href="#E14"]').first().click();
  await expect(page.locator("#E14")).toHaveAttribute("open", "");
  const evidence = await page.evaluate(() => ({
    images: [...document.images].map((image) => ({
      loaded: image.complete && image.naturalWidth > 0,
      inline: image.src.startsWith("data:"),
    })),
    missingAnchors: [...document.querySelectorAll('a[href^="#"]')]
      .filter((link) => !document.getElementById(link.hash.slice(1)))
      .map((link) => link.hash),
    desktopOverflow: document.documentElement.scrollWidth > innerWidth,
  }));
  expect(evidence.images.every((image) => image.loaded && image.inline)).toBe(
    true,
  );
  expect(evidence.missingAnchors).toEqual([]);
  expect(evidence.desktopOverflow).toBe(false);
  await page.locator("#scenario").selectOption("terminal");
  await page.locator(".steps button").nth(3).click();
  await page.locator('#stage a[href="#E3"]').click();
  await expect(page.locator("#E3")).toHaveAttribute("open", "");
  await page.evaluate(() => document.querySelector("main").scrollTo(0, 0));
  await page.screenshot({ path: `${proof}/page-desktop.png` });
  await page.locator("#conversation").scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${proof}/page-walkthrough.png` });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.evaluate(() => window.scrollTo(0, 0));
  const mobileOverflow = await page.evaluate(
    () => document.documentElement.scrollWidth > innerWidth,
  );
  expect(mobileOverflow).toBe(false);
  await page.screenshot({ path: `${proof}/page-mobile.png` });
  await page.locator("#G1").scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${proof}/page-mobile-gap.png` });
  expect(errors).toEqual([]);
  expect(network).toEqual([]);
  const result = {
    checked_at: new Date().toISOString(),
    browser: "Installed Microsoft Edge",
    evidence,
    mobileOverflow,
    scriptErrors: errors,
    externalRequests: network,
    interactions:
      "Step navigation; three scenarios; payload/reply update; failure branch; source disclosure",
  };
  await writeFile(
    `${proof}/page-verification.json`,
    JSON.stringify(result, null, 2) + "\n",
  );
  console.log(JSON.stringify(result, null, 2));
} finally {
  await browser.close();
}
