import { chromium, expect } from "@playwright/test";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";

const server = createServer((request, response) => {
  request.resume();
  response.setHeader("content-type", "application/json");
  response.end(
    JSON.stringify({
      choices: [
        {
          message: {
            content: JSON.stringify({
              text: "Handy",
              operation: "insert",
              effect: { type: "none" },
            }),
          },
        },
      ],
    }),
  );
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser
  .contexts()
  .flatMap((c) => c.pages())
  .find((p) => p.url() === "http://localhost:1420/");
let previous;
const invoke = (command, args) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const refresh = () =>
  page.evaluate(async () => {
    const urls = performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .filter((url) => url.includes("/src/stores/settingsStore.ts"));
    const url =
      urls.filter((url) => url.includes("?t=")).at(-1) ??
      urls[0] ??
      "/src/stores/settingsStore.ts";
    const { useSettingsStore } = await import(url);
    await useSettingsStore.getState().refreshSettings();
  });
try {
  if (!page) throw new Error("Native settings WebView not found");
  const actual = await invoke("get_app_dir_path");
  if (
    actual.toLowerCase() !==
    resolve("src-tauri/target/debug/Data").toLowerCase()
  )
    throw new Error("Refusing to mutate a non-test data directory");
  previous = await invoke("get_app_settings");
  let catalog = await invoke("get_context_profiles");
  if (process.argv.includes("--restart")) {
    const terminal = catalog.profiles.find(
      (p) => p.name === "Native UI Terminal",
    );
    const mail = catalog.profiles.find((p) => p.name === "Native UI Mail");
    if (
      terminal?.dictionary[0]?.canonical !== "Codex" ||
      terminal?.dictionary[0]?.misheard_forms.join("|") !== "codecks|code X" ||
      mail?.prompt !== "Use {{transcript}} for email."
    )
      throw new Error("Native UI changes did not persist");
    for (const profile of [terminal, mail]) {
      if (
        (await invoke("get_profile_memory", { profileId: profile.id }))
          .length !== 0
      )
        throw new Error("Memory should be empty after restart");
      catalog = await invoke("delete_context_profile", {
        id: profile.id,
        expectedRevision: catalog.revision,
      });
    }
    console.log(
      "PASS: two profiles, dictionary and custom prompt survived native restart; process memory empty; test profiles removed",
    );
  } else {
    if (catalog.profiles.some((p) => p.name.startsWith("Native UI ")))
      throw new Error("Prior Native UI fixtures require --restart cleanup");
    await invoke("set_post_process_provider", { providerId: "custom" });
    await invoke("change_post_process_base_url_setting", {
      providerId: "custom",
      baseUrl: `http://127.0.0.1:${server.address().port}`,
    });
    await invoke("change_post_process_model_setting", {
      providerId: "custom",
      model: "fixture",
    });
    await invoke("change_post_process_enabled_setting", { enabled: true });
    await refresh();
    await page.reload();
    await invoke("show_main_window_command");
    console.log(
      "Native viewport:",
      await page.evaluate(() => ({
        width: innerWidth,
        height: innerHeight,
        scale: devicePixelRatio,
      })),
    );
    await page
      .getByRole("button", { name: "Post Process", exact: true })
      .click();
    await page
      .getByRole("checkbox", { name: "Enable context profiles" })
      .focus();
    await page.keyboard.press("Space");
    await page.getByRole("button", { name: "Profiles", exact: true }).click();
    await page
      .getByRole("button", { name: "+ Add profile", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Profile name", exact: true })
      .fill("Native UI Terminal");
    await expect(
      page.getByRole("navigation").getByRole("button", {
        name: "Native UI Terminal WindowsTerminal.exe",
      }),
    ).toBeVisible();
    await page.getByRole("tab", { name: "Long-term dictionary" }).click();
    await page
      .getByRole("button", { name: "Add keyword", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Canonical keyword" })
      .fill("Codex");
    await page
      .getByRole("textbox", { name: "Misheard forms" })
      .fill("codecks\ncode X");
    await expect
      .poll(
        async () =>
          (await invoke("get_context_profiles")).profiles.find(
            (profile) => profile.name === "Native UI Terminal",
          ).dictionary[0]?.misheard_forms,
      )
      .toEqual(["codecks", "code X"]);
    await mkdir("design/proof", { recursive: true });
    await page.screenshot({
      path: "design/proof/profiles-native-dictionary.png",
    });
    await page
      .getByRole("button", { name: "+ Add profile", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Profile name", exact: true })
      .fill("Native UI Mail");
    await page
      .getByRole("textbox", { name: "Application executable", exact: true })
      .fill("mail.exe");
    await expect(
      page
        .getByRole("navigation")
        .getByRole("button", { name: "Native UI Mail mail.exe" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Customize", exact: true }).click();
    await page
      .getByRole("textbox", { name: "System prompt", exact: true })
      .fill("Use {{transcript}} for email.");
    await page
      .getByRole("button", { name: "Save and close", exact: true })
      .click();
    await expect(
      page.getByText("Custom prompt for this profile.", { exact: true }),
    ).toBeVisible();
    await page.screenshot({ path: "design/proof/profiles-native-context.png" });
    await invoke("change_post_process_enabled_setting", { enabled: false });
    await refresh();
    await expect(
      page.getByRole("button", { name: "Profiles", exact: true }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Post Process", exact: true }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("tablist", { name: "Profile settings" }),
    ).toHaveCount(0);
    console.log(
      "PASS: native settings UI, toggle, two profiles, dictionary, custom prompt, hidden-section navigation",
    );
  }
} finally {
  if (previous) {
    await invoke("change_post_process_profiles_setting", { enabled: false });
    await invoke("change_post_process_base_url_setting", {
      providerId: "custom",
      baseUrl: previous.post_process_providers.find((p) => p.id === "custom")
        .base_url,
    });
    await invoke("change_post_process_model_setting", {
      providerId: "custom",
      model: previous.post_process_models.custom ?? "",
    });
    await invoke("set_post_process_provider", {
      providerId: previous.post_process_provider_id,
    });
    await invoke("change_post_process_enabled_setting", {
      enabled: previous.post_process_enabled,
    });
    await refresh();
  }
  await browser.close();
  server.close();
}
