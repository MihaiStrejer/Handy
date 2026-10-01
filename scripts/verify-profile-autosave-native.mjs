import { chromium, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser
  .contexts()
  .flatMap((context) => context.pages())
  .find((page) => page.url() === "http://localhost:1420/");
const invoke = (command, args) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const name = `Autosave proof ${Date.now()}`;
let created;
try {
  expect((await invoke("get_app_dir_path")).toLowerCase()).toBe(
    resolve("src-tauri/target/debug/Data").toLowerCase(),
  );
  let catalog = await invoke("get_context_profiles");
  catalog = await invoke("save_context_profile", {
    expectedRevision: catalog.revision,
    profile: {
      id: "",
      name,
      revision: 0,
      dictionary_revision: 0,
      icon: "terminal",
      prompt: null,
      rules: [{ application: "autosave-proof.exe", workspace: null }],
      dictionary: [],
    },
  });
  created = catalog.profiles.find((profile) => profile.name === name).id;
  const stored = async () =>
    (await invoke("get_context_profiles")).profiles.find(
      (profile) => profile.id === created,
    );
  await page.reload();
  await invoke("show_main_window_command");
  await page.getByRole("button", { name: "Profiles", exact: true }).click();
  await page
    .getByRole("navigation", { name: "Context profiles" })
    .getByRole("button", { name: `${name} autosave-proof.exe`, exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill(`${name} renamed`);
  await expect.poll(async () => (await stored()).name).toBe(`${name} renamed`);
  await page.getByRole("radio", { name: "Chrome", exact: true }).check();
  await expect.poll(async () => (await stored()).icon).toBe("chrome");
  await expect(
    page.getByRole("button", { name: "Save profile", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  await page.getByRole("button", { name: "Add keyword", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Canonical keyword", exact: true })
    .fill("Codex");
  await page
    .getByRole("textbox", { name: "Misheard forms", exact: true })
    .fill("codecks");
  await expect
    .poll(async () => (await stored()).dictionary[0]?.misheard_forms)
    .toEqual(["codecks"]);
  await page.getByRole("tab", { name: "Context selection" }).click();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  const modal = page.getByRole("dialog", {
    name: "System prompt",
    exact: true,
  });
  const prompt = modal.getByRole("textbox", {
    name: "System prompt",
    exact: true,
  });
  await prompt.fill("Discarded native modal draft");
  await page.waitForTimeout(650);
  expect((await stored()).prompt).toBeNull();
  await modal
    .getByRole("button", { name: "Cancel", exact: true })
    .last()
    .click();
  expect((await stored()).prompt).toBeNull();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  await prompt.fill("{{unknown}}");
  await modal
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(modal.getByRole("alert")).toBeVisible();
  await prompt.fill("Rewrite {{transcript}} for the current workspace.");
  await mkdir("design/proof", { recursive: true });
  await page.screenshot({
    path: "design/proof/profile-prompt-modal-native.png",
  });
  await modal
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(modal).toHaveCount(0);
  expect((await stored()).prompt).toBe(
    "Rewrite {{transcript}} for the current workspace.",
  );
  await page.screenshot({ path: "design/proof/profile-autosave-native.png" });
  await writeFile(
    "design/proof/profile-autosave-native.json",
    JSON.stringify(
      {
        checked_at: new Date().toISOString(),
        autosave: ["name", "icon", "dictionary"],
        prompt: [
          "typing does not save",
          "cancel discards",
          "invalid prompt remains open",
          "save and close persists",
        ],
        test_data: "temporary profile removed in cleanup",
      },
      null,
      2,
    ) + "\n",
  );
  console.log("PASS: native profile autosave and transactional prompt modal");
} finally {
  try {
    await page.keyboard.press("Escape");
    await page
      .getByRole("button", { name: "General Fallback", exact: true })
      .click();
    const catalog = await invoke("get_context_profiles");
    if (created && catalog.profiles.some((profile) => profile.id === created))
      await invoke("delete_context_profile", {
        id: created,
        expectedRevision: catalog.revision,
      });
    await page.reload();
    await page.getByRole("button", { name: "Profiles", exact: true }).click();
  } finally {
    await browser.close();
  }
}
