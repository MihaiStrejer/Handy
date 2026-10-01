import { chromium, expect } from "@playwright/test";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { mkdir, writeFile } from "node:fs/promises";

let endpointRequests = 0;
const server = createServer((request, response) => {
  endpointRequests += 1;
  request.resume();
  response.writeHead(500).end("Profile setup must not call an endpoint");
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
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
let previous;
let createdId;
const name = `Offline setup check ${Date.now()}`;
try {
  if (!page) throw new Error("Native development WebView not found");
  expect((await invoke("get_app_dir_path")).toLowerCase()).toBe(
    resolve("src-tauri/target/debug/Data").toLowerCase(),
  );
  previous = await invoke("get_app_settings");
  await invoke("change_post_process_profiles_setting", { enabled: false });
  await invoke("set_post_process_provider", { providerId: "custom" });
  await invoke("change_post_process_base_url_setting", {
    providerId: "custom",
    baseUrl: `http://127.0.0.1:${server.address().port}`,
  });
  await invoke("change_post_process_model_setting", {
    providerId: "custom",
    model: "",
  });
  await invoke("change_post_process_api_key_setting", {
    providerId: "custom",
    apiKey: "",
  });
  await invoke("change_post_process_enabled_setting", { enabled: false });
  await expect(
    invoke("change_post_process_profiles_setting", { enabled: true }),
  ).rejects.toThrow("Enable post-processing");
  await invoke("change_post_process_enabled_setting", { enabled: true });
  await page.reload();
  await invoke("show_main_window_command");
  await page.getByRole("button", { name: "Post Process", exact: true }).click();
  await page.getByRole("checkbox", { name: "Enable context profiles" }).focus();
  await page.keyboard.press("Space");
  await page.getByRole("button", { name: "Profiles", exact: true }).click();
  await page
    .getByRole("button", { name: "+ Add profile", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill(name);
  await expect(
    page.getByRole("navigation").getByRole("button", {
      name: `${name} WindowsTerminal.exe`,
      exact: true,
    }),
  ).toBeVisible();
  let catalog = await invoke("get_context_profiles");
  createdId = catalog.profiles.find((profile) => profile.name === name).id;
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  await page.getByRole("button", { name: "Add keyword", exact: true }).click();
  await page.getByRole("textbox", { name: "Canonical keyword" }).fill("Codex");
  await page.getByRole("textbox", { name: "Misheard forms" }).fill("codecks");
  await expect
    .poll(async () => {
      catalog = await invoke("get_context_profiles");
      return catalog.profiles.find((profile) => profile.id === createdId)
        .dictionary;
    })
    .toMatchObject([{ canonical: "Codex", misheard_forms: ["codecks"] }]);
  const settings = await invoke("get_app_settings");
  expect(settings.post_process_models.custom).toBe("");
  expect(settings.post_process_api_keys.custom).toBe("");
  expect(settings.post_process_profiles).toBe(true);
  expect(endpointRequests).toBe(0);
  await mkdir("design/proof", { recursive: true });
  await page.screenshot({
    path: "design/proof/profile-setup-without-model.png",
  });
  // Even a configured model must not cause enable-time network traffic.
  await invoke("change_post_process_profiles_setting", { enabled: false });
  await invoke("change_post_process_model_setting", {
    providerId: "custom",
    model: "deliberately-unreachable-model",
  });
  await invoke("change_post_process_profiles_setting", { enabled: true });
  expect(endpointRequests).toBe(0);
  const result = {
    checked_at: new Date().toISOString(),
    native: true,
    no_model_or_key: "enable, create profile and save dictionary passed",
    configured_model: "enable succeeded with a rejecting endpoint",
    parent_flag: "still required",
    endpoint_requests: endpointRequests,
  };
  await writeFile(
    "design/proof/profile-setup-without-model.json",
    JSON.stringify(result, null, 2) + "\n",
  );
  console.log(result);
} finally {
  try {
    if (previous) {
      const catalog = await invoke("get_context_profiles");
      const created = catalog.profiles.find(
        (profile) => profile.id === createdId || profile.name === name,
      );
      if (created)
        await invoke("delete_context_profile", {
          id: created.id,
          expectedRevision: catalog.revision,
        });
      await invoke("change_post_process_profiles_setting", { enabled: false });
      await invoke("change_post_process_base_url_setting", {
        providerId: "custom",
        baseUrl: previous.post_process_providers.find(
          (provider) => provider.id === "custom",
        ).base_url,
      });
      await invoke("change_post_process_model_setting", {
        providerId: "custom",
        model: previous.post_process_models.custom ?? "",
      });
      await invoke("change_post_process_api_key_setting", {
        providerId: "custom",
        apiKey: previous.post_process_api_keys.custom ?? "",
      });
      await invoke("set_post_process_provider", {
        providerId: previous.post_process_provider_id,
      });
      await invoke("change_post_process_enabled_setting", {
        enabled: previous.post_process_enabled,
      });
      await invoke("change_post_process_profiles_setting", {
        enabled: previous.post_process_profiles,
      });
      await page.reload();
    }
  } finally {
    await browser.close();
    server.close();
  }
}
