import { chromium, expect } from "@playwright/test";
import { createServer } from "node:http";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { execFileSync } from "node:child_process";

const directory = "design/proof/architecture";
await mkdir(directory, { recursive: true });
const server = createServer((request, response) => {
  request.resume();
  response.setHeader("content-type", "application/json");
  response.end(JSON.stringify({ choices: [{ message: { content: '{"text":"Handy","operation":"insert","effect":{"type":"none"}}' } }] }));
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = browser.contexts().flatMap(context => context.pages());
const page = pages.find(page => page.url() === "http://localhost:1420/");
const overlay = pages.find(page => page.url().includes("/src/overlay/index.html"));
let previous;
let created;
const invoke = (command, args) => page.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), { command, args });
try {
  const directoryInUse = await invoke("get_app_dir_path");
  if (directoryInUse.toLowerCase() !== resolve("src-tauri/target/debug/Data").toLowerCase()) throw new Error("Capture requires the isolated development instance");
  previous = await invoke("get_app_settings");
  if (previous.post_process_profiles) throw new Error("Capture fixture expects profiles disabled");
  await invoke("set_post_process_provider", { providerId: "custom" });
  await invoke("change_post_process_base_url_setting", { providerId: "custom", baseUrl: `http://127.0.0.1:${server.address().port}` });
  await invoke("change_post_process_model_setting", { providerId: "custom", model: "capture-fixture" });
  await invoke("change_post_process_enabled_setting", { enabled: true });
  await invoke("change_post_process_profiles_setting", { enabled: true });
  let catalog = await invoke("get_context_profiles");
  catalog = await invoke("save_context_profile", { expectedRevision: catalog.revision, profile: { id: "", name: "Architecture terminal", revision: 0, dictionary_revision: 0, icon: "terminal", prompt: null, rules: [{ application: "WindowsTerminal.exe", workspace: null }], dictionary: [{ id: "codex", canonical: "Codex", misheard_forms: ["codecks", "code X"] }, { id: "claude", canonical: "Claude Code", misheard_forms: ["cloud code"] }] } });
  created = catalog.profiles.find(profile => profile.name === "Architecture terminal").id;
  await page.reload();
  await invoke("show_main_window_command");
  await page.getByRole("button", { name: "Profiles", exact: true }).click();
  await page.getByRole("navigation").getByRole("button", { name: "Architecture terminal WindowsTerminal.exe" }).click();
  await expect(page.getByText("Inherited from General.", { exact: false })).toBeVisible();
  await page.screenshot({ path: `${directory}/context-original.png` });
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  await expect(page.getByRole("textbox", { name: "Canonical keyword" }).first()).toHaveValue("Codex");
  await page.screenshot({ path: `${directory}/dictionary-original.png` });
  await page.evaluate(() => {
    document.querySelectorAll(".asc-annot").forEach(element => element.remove());
    const marks = [[document.querySelector(".profile-selector"), 1, "Profile being edited"], [document.querySelector(".profile-tabs"), 2, "Three profile tabs"], [document.querySelector(".profile-keyword"), 3, "Keyword → mishearings"]];
    const color = "#1E5A8A";
    marks.forEach(([element, number, label], index) => {
      const rect = element.getBoundingClientRect();
      const box = document.createElement("div"); box.className = "asc-annot";
      Object.assign(box.style, { position: "fixed", left: `${rect.left - 3}px`, top: `${rect.top - 3}px`, width: `${rect.width + 6}px`, height: `${rect.height + 6}px`, border: `2px solid ${color}`, borderRadius: "5px", zIndex: 9998, pointerEvents: "none" }); document.body.append(box);
      const badge = document.createElement("div"); badge.className = "asc-annot"; badge.textContent = number;
      Object.assign(badge.style, { position: "fixed", left: `${rect.left - 9}px`, top: `${rect.top - 9}px`, width: "20px", height: "20px", lineHeight: "20px", borderRadius: "50%", background: color, color: "white", fontSize: "12px", textAlign: "center", zIndex: 9999 }); document.body.append(badge);
      const legend = document.createElement("div"); legend.className = "asc-annot"; legend.textContent = `${number}  ${label}`;
      Object.assign(legend.style, { position: "fixed", left: "8px", bottom: `${48 + (2 - index) * 39}px`, width: "130px", minHeight: "32px", background: color, color: "white", font: "11px/1.3 Segoe UI", padding: "5px", borderRadius: "4px", zIndex: 9999 }); document.body.append(legend);
    });
  });
  await page.screenshot({ path: `${directory}/dictionary-annotated.png` });
  await page.evaluate(() => document.querySelectorAll(".asc-annot").forEach(element => element.remove()));
  const display = { session_id: "architecture-fixture", enabled: true, profile: { name: "Architecture terminal", icon: "terminal", input_mode: "unknown" } };
  await invoke("plugin:event|emit", { event: "show-overlay", payload: { state: "recording", context: display } });
  await invoke("plugin:event|emit", { event: "recording-ready", payload: null });
  await expect(overlay.getByRole("img", { name: "Architecture terminal: Selection unavailable" })).toBeVisible();
  await overlay.screenshot({ path: `${directory}/overlay.png` });
  await invoke("plugin:event|emit", { event: "hide-overlay", payload: null });
  const metadata = { captured_at: new Date().toISOString(), commit: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(), branch: execFileSync("git", ["branch", "--show-current"], { encoding: "utf8" }).trim(), instance: "isolated portable development data; no tenant", note: "Actual native UI with synthetic profile data. Overlay uses a session event fixture; no voice or external API request.", viewport: await page.evaluate(() => ({ width: innerWidth, height: innerHeight, scale: devicePixelRatio })) };
  await writeFile(`${directory}/capture.json`, JSON.stringify(metadata, null, 2) + "\n");
  console.log(metadata);
} finally {
  if (created) {
    const catalog = await invoke("get_context_profiles");
    await invoke("delete_context_profile", { id: created, expectedRevision: catalog.revision });
  }
  if (previous) {
    await invoke("change_post_process_profiles_setting", { enabled: false });
    await invoke("change_post_process_base_url_setting", { providerId: "custom", baseUrl: previous.post_process_providers.find(profile => profile.id === "custom").base_url });
    await invoke("change_post_process_model_setting", { providerId: "custom", model: previous.post_process_models.custom ?? "" });
    await invoke("set_post_process_provider", { providerId: previous.post_process_provider_id });
    await invoke("change_post_process_enabled_setting", { enabled: previous.post_process_enabled });
    await page.reload();
  }
  await browser.close(); server.close();
}
