import { chromium, expect } from "@playwright/test";
import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
try {
  const pages = browser.contexts().flatMap((context) => context.pages());
  const main = pages.find((page) => page.url() === "http://localhost:1420/");
  const overlay = pages.find((page) =>
    page.url().includes("/src/overlay/index.html"),
  );
  if (!main || !overlay) throw new Error("Native WebViews are unavailable");
  const data = await main.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_app_dir_path"),
  );
  if (
    data.toLowerCase() !== resolve("src-tauri/target/debug/Data").toLowerCase()
  )
    throw new Error("Refusing to use a non-test instance");
  const emit = (event, payload) =>
    main.evaluate(
      ({ event, payload }) =>
        window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {
          event,
          payload,
        }),
      { event, payload },
    );
  const context = {
    session_id: "fixture",
    enabled: true,
    profile: { name: "Handy workspace", icon: "terminal", input_mode: "edit" },
  };
  await emit("show-overlay", { state: "recording", context });
  await expect(
    overlay.getByRole("img", { name: "Handy workspace: Edit selection" }),
  ).toBeVisible();
  await emit("recording-ready", null);
  await expect(overlay.locator(".sprofile.ready")).toBeVisible();
  await expect(overlay.locator(".sdot")).toHaveCount(0);
  const bounds = await overlay.evaluate(() => {
    const rect = document.querySelector(".sprofile").getBoundingClientRect();
    return {
      width: innerWidth,
      height: innerHeight,
      scale: devicePixelRatio,
      iconFits:
        rect.x >= 0 &&
        rect.y >= 0 &&
        rect.right <= innerWidth &&
        rect.bottom <= innerHeight,
    };
  });
  if (!bounds.iconFits) throw new Error("Profile icon exceeds native bounds");
  await mkdir("design/proof", { recursive: true });
  await overlay.screenshot({ path: "design/proof/profile-overlay-native.png" });
  await emit("show-overlay", { state: "processing", context });
  await expect(overlay.locator(".sspinner")).toBeVisible();
  await emit("hide-overlay", null);
  await expect(overlay.locator(".sprofile")).toHaveCount(0);
  console.log(
    "PASS: native overlay event fixture (not a voice session)",
    bounds,
  );
} finally {
  await browser.close();
}
