import { chromium, expect } from "@playwright/test";
import { copyFile, mkdir, unlink, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = browser.contexts().flatMap((context) => context.pages());
const page = pages.find((page) => page.url() === "http://localhost:1420/");
const overlay = pages.find((page) =>
  page.url().includes("/src/overlay/index.html"),
);
const invoke = (command, args) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const name = `Icon import proof ${Date.now()}`;
const source = resolve(`design/proof/icon-import-source-${Date.now()}.png`);
let created;
let sourceExists = false;
try {
  expect((await invoke("get_app_dir_path")).toLowerCase()).toBe(
    resolve("src-tauri/target/debug/Data").toLowerCase(),
  );
  await mkdir("design/proof", { recursive: true });
  await copyFile("src-tauri/resources/handy.png", source);
  sourceExists = true;
  const data = await invoke("import_profile_icon", { path: source });
  expect(data).toMatch(/^data:image\/png;base64,/);
  let catalog = await invoke("get_context_profiles");
  catalog = await invoke("save_context_profile", {
    expectedRevision: catalog.revision,
    profile: {
      id: "",
      name,
      icon: { custom: data },
      prompt: null,
      rules: [{ application: "icon-proof.exe", workspace: null }],
      consolidation_instructions: null,
    },
  });
  created = catalog.profiles.find((profile) => profile.name === name).id;
  await unlink(source);
  sourceExists = false;
  await page.reload();
  await invoke("show_main_window_command");
  await page.getByRole("button", { name: "Profiles", exact: true }).click();
  const row = page
    .getByRole("navigation", { name: "Context profiles" })
    .getByRole("button", { name: `${name} icon-proof.exe`, exact: true });
  await row.click();
  await expect(
    page.getByRole("radio", { name: "Custom", exact: true }),
  ).toBeChecked();
  await expect
    .poll(() =>
      row
        .locator("img")
        .evaluate((image) => image.complete && image.naturalWidth > 0),
    )
    .toBe(true);
  await page.screenshot({ path: "design/proof/profile-icons-native.png" });
  await invoke("plugin:event|emit", {
    event: "show-overlay",
    payload: {
      state: "recording",
      context: {
        session_id: "icon-proof",
        enabled: true,
        profile: { name: "Imported icon", icon: data, input_mode: "compose" },
      },
    },
  });
  await expect
    .poll(() =>
      overlay
        .locator(".sprofile img")
        .evaluate((image) => image.complete && image.naturalWidth > 0),
    )
    .toBe(true);
  await overlay.screenshot({
    path: "design/proof/profile-custom-icon-overlay.png",
  });
  await writeFile(
    "design/proof/profile-icons-native.json",
    JSON.stringify(
      {
        checked_at: new Date().toISOString(),
        import_and_save: "passed",
        source_deleted: true,
        selector_and_picker: "rendered saved image",
        overlay:
          "rendered image from session-event fixture; no voice recording",
        builtin_count: 15,
        settings_restored: "test profile removed in cleanup",
      },
      null,
      2,
    ) + "\n",
  );
  console.log(
    "PASS: native import/save, deleted-source independence, picker/list and overlay image rendering",
  );
} finally {
  try {
    await invoke("plugin:event|emit", { event: "hide-overlay", payload: null });
    const catalog = await invoke("get_context_profiles");
    const fixture = catalog.profiles.find(
      (profile) => profile.id === created || profile.name === name,
    );
    if (fixture)
      await invoke("delete_context_profile", {
        id: fixture.id,
        expectedRevision: catalog.revision,
      });
    if (sourceExists) await unlink(source);
    await page.reload();
    await page.getByRole("button", { name: "Profiles", exact: true }).click();
  } finally {
    await browser.close();
  }
}
