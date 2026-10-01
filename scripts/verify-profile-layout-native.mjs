import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
try {
  const page = browser
    .contexts()
    .flatMap((context) => context.pages())
    .find((page) => page.url() === "http://localhost:1420/");
  if (!page)
    throw new Error("Native development settings window is unavailable");
  const directory = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_app_dir_path"),
  );
  expect(directory.toLowerCase()).toBe(
    resolve("src-tauri/target/debug/Data").toLowerCase(),
  );
  await page.getByRole("button", { name: "Profiles", exact: true }).click();
  await expect(
    page.getByRole("radiogroup", { name: "Profile icon" }),
  ).toBeVisible();
  await expect(page.getByRole("radio")).toHaveCount(15);
  const metrics = await page.evaluate(() => {
    const section = document
      .querySelector(".context-profiles")
      .getBoundingClientRect();
    const sidebar = document
      .querySelector(".profile-selector")
      .getBoundingClientRect();
    const footer = document
      .querySelector(".h-screen > .border-t")
      .getBoundingClientRect();
    return {
      viewport: {
        width: innerWidth,
        height: innerHeight,
        scale: devicePixelRatio,
      },
      sectionBottom: section.bottom,
      sidebarBottom: sidebar.bottom,
      sidebarHeight: sidebar.height,
      footerTop: footer.top,
      overflow: document.documentElement.scrollWidth > innerWidth,
    };
  });
  expect(
    Math.abs(metrics.sectionBottom - metrics.sidebarBottom),
  ).toBeLessThanOrEqual(1);
  expect(metrics.sidebarHeight).toBeGreaterThan(metrics.viewport.height - 70);
  expect(
    Math.abs(metrics.sidebarBottom - metrics.footerTop),
  ).toBeLessThanOrEqual(1);
  expect(metrics.overflow).toBe(false);
  await mkdir("design/proof", { recursive: true });
  await page.screenshot({ path: "design/proof/profiles-layout-native.png" });
  await writeFile(
    "design/proof/profiles-layout-native.json",
    JSON.stringify(
      {
        checked_at: new Date().toISOString(),
        ...metrics,
        note: "Read-only native layout check; no profiles or preferences edited. Many-profile scrolling and icon persistence are checked with browser fixtures.",
      },
      null,
      2,
    ) + "\n",
  );
  console.log(metrics);
} finally {
  await browser.close();
}
