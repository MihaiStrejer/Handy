import { chromium } from "@playwright/test";
import { resolve } from "node:path";
import { createServer } from "node:http";
const endpoint = createServer((request, response) => {
  request.resume();
  response.setHeader("content-type", "application/json");
  response.end(
    JSON.stringify({
      choices: [
        {
          message: {
            content:
              '{"text":"Handy","operation":"insert","effect":{"type":"none"}}',
          },
        },
      ],
    }),
  );
});
await new Promise((resolve) => endpoint.listen(0, "127.0.0.1", resolve));
const expectedData = resolve("src-tauri/target/debug/Data").toLowerCase();
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
let previous;
let page;
try {
  page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url() === "http://localhost:1420/");
  if (!page) throw new Error("Native settings WebView not found");
  const actualData = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_app_dir_path"),
  );
  if (actualData.toLowerCase() !== expectedData)
    throw new Error("Refusing to mutate a non-test data directory");
  previous = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_app_settings"),
  );
  await page.evaluate(async (port) => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    await invoke("set_post_process_provider", { providerId: "custom" });
    await invoke("change_post_process_base_url_setting", {
      providerId: "custom",
      baseUrl: `http://127.0.0.1:${port}`,
    });
    await invoke("change_post_process_model_setting", {
      providerId: "custom",
      model: "fixture",
    });
  }, endpoint.address().port);
  console.log(
    await page.evaluate(async () => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const assert = (value, message) => {
        if (!value) throw new Error(message);
      };
      const reject = async (command, args) => {
        let failed = false;
        try {
          await invoke(command, args);
        } catch {
          failed = true;
        }
        assert(failed, `${command} should reject`);
      };
      await invoke("change_post_process_enabled_setting", { enabled: false });
      await reject("change_post_process_profiles_setting", { enabled: true });
      let catalog = await invoke("get_context_profiles");
      let general = catalog.profiles.find((p) => p.id === "general");
      general.prompt = "Rewrite {{transcript}} using {{dictionary}}.";
      catalog = await invoke("save_context_profile", {
        profile: general,
        expectedRevision: catalog.revision,
      });
      await reject("delete_context_profile", {
        id: "general",
        expectedRevision: catalog.revision,
      });
      if (catalog.profiles.some((p) => p.name.startsWith("Native test")))
        throw new Error("Remove the prior test fixture before rerunning");
      let profile = {
        id: "",
        name: "Native test profile",
        revision: 0,
        dictionary_revision: 0,
        icon: "terminal",
        rules: [{ application: "WindowsTerminal.exe", workspace: null }],
        prompt: null,
        dictionary: [
          { id: "codex", canonical: "Codex", misheard_forms: ["codecks"] },
        ],
      };
      catalog = await invoke("save_context_profile", {
        profile,
        expectedRevision: catalog.revision,
      });
      profile = catalog.profiles.find((p) => p.name === "Native test profile");
      const stale = catalog.revision;
      profile.name = "Native test renamed";
      profile.prompt = "Custom {{transcript}}";
      catalog = await invoke("save_context_profile", {
        profile,
        expectedRevision: catalog.revision,
      });
      await reject("save_context_profile", {
        profile,
        expectedRevision: stale,
      });
      profile = catalog.profiles.find((p) => p.id === profile.id);
      const collision = structuredClone(profile);
      collision.dictionary.push({
        id: "other",
        canonical: "Other",
        misheard_forms: [" CODECKS "],
      });
      await reject("save_context_profile", {
        profile: collision,
        expectedRevision: catalog.revision,
      });
      profile.prompt = null;
      catalog = await invoke("save_context_profile", {
        profile,
        expectedRevision: catalog.revision,
      });
      await invoke("change_post_process_enabled_setting", { enabled: true });
      await invoke("change_post_process_profiles_setting", { enabled: true });
      let settings = await invoke("get_app_settings");
      assert(
        settings.post_process_enabled && settings.post_process_profiles,
        "both flags enabled",
      );
      await invoke("change_post_process_enabled_setting", { enabled: false });
      settings = await invoke("get_app_settings");
      assert(
        !settings.post_process_enabled && !settings.post_process_profiles,
        "both flags disabled",
      );
      const reloaded = await invoke("get_context_profiles");
      const retained = reloaded.profiles.find((p) => p.id === profile.id);
      assert(
        retained.prompt === null &&
          retained.dictionary[0].canonical === "Codex",
        "retained data",
      );
      assert(
        (await invoke("get_profile_memory", { profileId: profile.id }))
          .length === 0,
        "memory is empty",
      );
      return {
        status: "passed",
        profileId: profile.id,
        revision: reloaded.revision,
        checks: [
          "create",
          "rename",
          "override",
          "reset",
          "collision",
          "stale edit",
          "General protected",
          "flag dependency",
          "atomic clear",
          "retained dictionary",
          "memory separate",
        ],
      };
    }),
  );
} finally {
  if (previous)
    await page.evaluate(async (previous) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
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
    }, previous);
  await browser.close();
  endpoint.close();
}
