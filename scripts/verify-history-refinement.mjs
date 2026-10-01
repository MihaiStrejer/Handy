import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";

const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({ viewport: { width: 1160, height: 900 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const cases = {
  rate_limit: "The provider rate limit was reached.",
  auth: "The provider rejected the API key.",
  timeout: "No complete response arrived within 60 seconds.",
  invalid: "The response did not contain a valid rewrite.",
  setup: "Select a post-processing model to run this profile.",
};
await mkdir("design/directions/proof/round-2", { recursive: true });
try {
  await page.goto("http://localhost:1420/design/directions/round-2.html");
  const option = page.locator("#B");
  const entry = option.locator(".entry").first();
  await expect(entry.locator(".entry-top .status")).toHaveText("Post-processed");
  const time = await entry.locator("time").boundingBox();
  const status = await entry.locator(".status").boundingBox();
  expect(Math.abs(time.y - status.y)).toBeLessThan(5);
  expect(status.x).toBeGreaterThan(time.x + time.width);
  await option.locator(".demo").screenshot({ path: "design/directions/proof/round-2/success.png" });
  for (const [state, reason] of Object.entries(cases)) {
    await option.locator("[data-state]").selectOption(state);
    await expect(entry.locator(".entry-top .status")).toHaveText("Post-process failed");
    await expect(entry.locator(".processing-summary .entry-reason")).toHaveText(reason);
    await expect(option.locator(".failure-explanation")).toContainText("Original transcript saved");
    await option.locator(".demo").screenshot({ path: `design/directions/proof/round-2/${state}.png` });
    await option.getByRole("tab", { name: "Calls", exact: true }).click();
    await expect(option.getByRole("tabpanel")).toContainText(state === "setup" ? "No provider calls" : "1 · Rewrite");
    await option.getByRole("tab", { name: "Requests", exact: true }).click();
    await expect(option.getByRole("tabpanel")).toContainText(state === "setup" ? "No request was sent" : "Ask codecks");
    await page.keyboard.press("Escape");
    await expect(option.locator(".detail-trigger")).toBeFocused();
    await expect(entry.locator(".processing-summary .entry-reason")).toBeVisible();
  }
  await page.locator('[data-sample="timeout"]').click();
  await expect(option.locator("[data-state]")).toHaveValue("timeout");
  await expect(option.locator(".failure-explanation h3")).toHaveText("Request timed out");
  await page.setViewportSize({ width: 680, height: 760 });
  for (const theme of ["dark", "light"]) {
    if (theme === "light") await page.getByRole("button", { name: "Switch to light" }).click();
    await option.locator("[data-state]").selectOption("rate_limit");
    await option.locator(".demo").screenshot({ path: `design/directions/proof/round-2/compact-inspector-${theme}.png` });
    await option.getByRole("button", { name: "Close processing details" }).click();
    await expect(entry.locator(".processing-summary .entry-reason")).toBeVisible();
    await option.locator(".demo").screenshot({ path: `design/directions/proof/round-2/compact-entry-${theme}.png` });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(await option.locator(".content").evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
  }
  expect(errors).toEqual([]);
  await writeFile("design/directions/proof/round-2/check.json", JSON.stringify({ checked_at: new Date().toISOString(), layout: "B", failure_cases: Object.keys(cases), checks: ["status beside timestamp", "reason inside metadata section", "reason visible with inspector closed", "failure explanation and next step", "no request for setup failure", "request and call tabs", "Escape restores focus", "gallery navigation", "680px dark/light overflow", "no page errors"] }, null, 2));
  console.log("PASS: selected inspector, header status and five failure designs");
} finally {
  await browser.close();
}
