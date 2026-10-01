import { chromium } from "@playwright/test";
import { resolve } from "node:path";
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
try {
  const page = browser
    .contexts()
    .flatMap((c) => c.pages())
    .find((p) => p.url() === "http://localhost:1420/");
  const actualData = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("get_app_dir_path"),
  );
  if (
    actualData.toLowerCase() !==
    resolve("src-tauri/target/debug/Data").toLowerCase()
  )
    throw Error("Refusing to mutate a non-test data directory");
  console.log(
    await page.evaluate(async () => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const catalog = await invoke("get_context_profiles");
      const profile = catalog.profiles.find(
        (p) => p.name === "Native test renamed",
      );
      if (
        !profile ||
        profile.name !== "Native test renamed" ||
        profile.prompt !== null ||
        profile.dictionary[0].canonical !== "Codex"
      )
        throw Error("Restart lost profile data");
      const settings = await invoke("get_app_settings");
      if (settings.post_process_enabled || settings.post_process_profiles)
        throw Error("Restart lost flag transition");
      const deleted = await invoke("delete_context_profile", {
        id: profile.id,
        expectedRevision: catalog.revision,
      });
      if (deleted.profiles.some((p) => p.id === profile.id))
        throw Error("Delete failed");
      return {
        restart: "passed",
        delete: "passed",
        memory: (await invoke("get_profile_memory", { profileId: profile.id }))
          .length,
      };
    }),
  );
} finally {
  await browser.close();
}
