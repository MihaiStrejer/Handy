import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";

const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({ viewport: { width: 1160, height: 920 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
await mkdir("design/directions/proof", { recursive: true });
try {
  await page.goto("http://localhost:1420/design/directions/round-1.html");
  for (const id of ["A", "B", "C"]) {
    const option = page.locator(`#${id}`);
    await option.getByRole("button", { name: "Processing details", exact: true }).click();
    await expect(option.getByRole("tabpanel")).toContainText("0.001376");
    await option.locator(".demo").screenshot({ path: `design/directions/proof/history-${id}-overview.png` });
    await option.getByRole("tab", { name: "Requests", exact: true }).click();
    await expect(option.getByRole("tabpanel")).toContainText("Ask codecks");
    await option.locator(".demo").screenshot({ path: `design/directions/proof/history-${id}-requests.png` });
    await option.getByLabel("Request call").selectOption("probe");
    await expect(option.getByRole("tabpanel")).toContainText("Return text exactly Handy");
    await option.getByRole("tab", { name: "Requests", exact: true }).focus();
    await page.keyboard.press("ArrowRight");
    await expect(option.getByRole("tab", { name: "Calls", exact: true })).toBeFocused();
    await expect(option.getByRole("tabpanel")).toContainText("2 calls");
    await page.keyboard.press("Escape");
    await expect(option.locator(".detail-trigger")).toBeFocused();
    await expect(option.locator(".detail")).toHaveCount(0);
    for (const state of ["failed", "legacy"]) {
      await option.locator("[data-state]").selectOption(state);
      await option.locator(".detail-trigger").click();
      await option.getByRole("tab", { name: "Overview", exact: true }).click();
      await expect(option.getByRole("tabpanel")).toContainText(state === "failed" ? "Unavailable" : "Details not recorded");
      if (id === "A") await option.locator(".demo").screenshot({ path: `design/directions/proof/history-A-${state}.png` });
      await page.keyboard.press("Escape");
    }
    await option.locator("[data-state]").selectOption("success");
  }
  await page.setViewportSize({ width: 680, height: 760 });
  for (const theme of ["dark", "light"]) {
    if (theme === "light") await page.getByRole("button", { name: "Switch to light" }).click();
    for (const id of ["A", "B", "C"]) {
      const option = page.locator(`#${id}`);
      await option.locator(".detail-trigger").click();
      await option.getByRole("tab", { name: "Requests", exact: true }).click();
      expect(await option.locator(".content").evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
      expect(await option.locator(".detail").evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
      await option.locator(".demo").screenshot({ path: `design/directions/proof/history-${id}-compact-${theme}.png` });
      await page.keyboard.press("Escape");
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  expect(errors).toEqual([]);
  await writeFile("design/directions/proof/history-design-check.json", JSON.stringify({ checked_at: new Date().toISOString(), options: ["A", "B", "C"], states: ["success", "failed", "legacy"], tabs: ["overview", "requests", "calls"], checks: ["request selector", "tab keyboard navigation", "Escape and focus restoration", "680px dark/light horizontal overflow", "no page errors"], real_api_calls: 0 }, null, 2));
  console.log("PASS: three design options, request inspection, state variants, keyboard navigation and compact dark/light layout");
} finally {
  await browser.close();
}
