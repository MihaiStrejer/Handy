import { chromium, expect } from "@playwright/test";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { copyFileSync, existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const phase = process.argv[2] ?? "backend";
const proof = resolve("design/proof/profile-memory");
await mkdir(proof, { recursive: true });
if (phase === "backend") {
  if (process.platform === "win32") {
    for (const name of [
      "DirectML.dll",
      "transcribe.dll",
      "ggml.dll",
      "ggml-base.dll",
      "ggml-cpu.dll",
    ]) {
      const source = resolve("src-tauri/target/debug", name);
      if (existsSync(source))
        copyFileSync(source, resolve("src-tauri/target/debug/deps", name));
    }
  }
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--lib",
      "context_profiles::",
      "--",
      "--nocapture",
    ],
    { encoding: "utf8", env: process.env },
  );
  if (result.error) throw result.error;
  const output = `${result.stdout}\n${result.stderr}`;
  const verdict = output.match(/test result: (.+)/)?.[0] ?? "No test result";
  await writeFile(
    resolve(proof, "native-backend.json"),
    JSON.stringify(
      {
        checked_at: new Date().toISOString(),
        command:
          "cargo test --manifest-path src-tauri/Cargo.toml --lib context_profiles:: -- --nocapture",
        result: verdict,
        windows_native_edit:
          process.platform === "win32" &&
          output.includes(
            "native_corrected_selection_is_verified_before_memory_reaches_next_request ... ok",
          ),
        scope:
          "Owned offscreen Edit controls, real local HTTP fixtures, local migration/session/admission/consolidation/recovery tests; no microphone, user clipboard or real model.",
        exit_code: result.status,
      },
      null,
      2,
    ) + "\n",
  );
  console.log(verdict);
  if (result.status !== 0) console.error(output);
  process.exitCode = result.status ?? 1;
} else if (["ui", "restart"].includes(phase)) {
  const data = resolve("src-tauri/target/debug/Data");
  await readFile(resolve(data, ".profile-memory-verification-owned"));
  const browser = await chromium.connectOverCDP(
    process.env.PROFILE_MEMORY_CDP ?? "http://127.0.0.1:9223",
  );
  let requests = 0;
  const server = createServer((request, response) => {
    requests++;
    request.resume();
    response.writeHead(500).end("Owned fixture: no provider call expected");
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  const page = browser
    .contexts()
    .flatMap((context) => context.pages())
    .find((page) => page.url() === "http://localhost:1420/");
  if (!page)
    throw new Error("Start the isolated native development app and Vite first");
  const invoke = (command, args) =>
    page.evaluate(
      ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
      { command, args },
    );
  let previous;
  try {
    expect((await invoke("get_app_dir_path")).toLowerCase()).toBe(
      data.toLowerCase(),
    );
    previous = await invoke("get_app_settings");
    await page.getByRole("button", { name: "Profiles", exact: true }).click();
    let catalog = await invoke("get_context_profiles");
    expect(catalog.schema_version).toBe(3);
    const general = catalog.profiles.find(
      (profile) => profile.id === "general",
    );
    const mail = catalog.profiles.find((profile) => profile.id === "profile-1");
    expect(general.long_term_memory).toContain('"Codex"');
    expect(general.long_term_memory).toContain('"codecks"');
    expect(mail.long_term_memory).toContain('"banque"');
    expect(mail.long_term_memory).not.toContain("Codex");
    expect(general.prompt).toContain("{{long_term_memory}}");
    expect(
      await invoke("get_profile_memory", { profileId: general.id }),
    ).toEqual([]);
    const backup = await readFile(
      resolve(data, "context-profiles.v1.backup.json"),
    );
    expect(
      backup.equals(
        await readFile("tests/fixtures/profile-memory-legacy-catalog.json"),
      ),
    ).toBe(true);
    await page
      .getByRole("tab", { name: "Long-term memory", exact: true })
      .click();
    await expect(
      page.locator('section[aria-label="Long-term memory"] pre'),
    ).toContainText("Codex");
    const instructions = page.getByRole("textbox", {
      name: "Consolidation instructions",
      exact: true,
    });
    await expect(instructions).not.toBeEmpty();
    const update = page.getByRole("button", {
      name: "Update long-term memory",
      exact: true,
    });
    await expect(update).toBeDisabled();
    const inputBounds = await instructions.boundingBox();
    const buttonBounds = await update.boundingBox();
    expect(buttonBounds.x).toBeGreaterThanOrEqual(
      inputBounds.x + inputBounds.width,
    );
    await expect(
      page.getByRole("button", { name: "Add keyword", exact: true }),
    ).toHaveCount(0);
    const edit = ({
      id,
      name,
      icon,
      rules,
      prompt,
      consolidation_instructions,
    }) => ({ id, name, icon, rules, prompt, consolidation_instructions });
    if (phase === "ui") {
      const dto = edit(general);
      await expect(
        invoke("save_context_profile", {
          expectedRevision: catalog.revision,
          profile: { ...dto, long_term_memory: "forbidden injection" },
        }),
      ).rejects.toBeTruthy();
      const expected = "Keep existing terms and explicit translation scopes.";
      await instructions.fill(expected);
      await expect
        .poll(
          async () =>
            (await invoke("get_context_profiles")).profiles.find(
              (p) => p.id === general.id,
            ).consolidation_instructions,
        )
        .toBe(expected);
    } else {
      await expect(instructions).toHaveValue(
        "Keep existing terms and explicit translation scopes.",
      );
    }
    await invoke("set_post_process_provider", { providerId: "custom" });
    await invoke("change_post_process_base_url_setting", {
      providerId: "custom",
      baseUrl: `http://127.0.0.1:${server.address().port}`,
    });
    await invoke("change_post_process_model_setting", {
      providerId: "custom",
      model: "owned-empty-memory-fixture",
    });
    await expect(
      invoke("start_profile_consolidation", {
        profileId: general.id,
        instructions: "Keep terms.",
      }),
    ).rejects.toThrow("empty_short_term");
    expect(requests).toBe(0);
    catalog = await invoke("get_context_profiles");
    expect(
      catalog.profiles.find((p) => p.id === general.id).long_term_memory,
    ).toBe(general.long_term_memory);
    await page.screenshot({ path: resolve(proof, `native-${phase}.png`) });
    await writeFile(
      resolve(proof, `native-${phase}.json`),
      JSON.stringify(
        {
          checked_at: new Date().toISOString(),
          schema: catalog.schema_version,
          migrated_literals: true,
          exact_backup_sha256: createHash("sha256")
            .update(backup)
            .digest("hex"),
          two_profile_isolation: true,
          narrow_save_rejects_memory: phase === "ui",
          instruction_persistence: true,
          button_right_of_input: true,
          empty_memory_rejects_without_http: true,
          endpoint_requests: requests,
          scope:
            "Real Tauri commands/store and native WebView UI. No microphone or model-driven promotion; those remain separate verification gates.",
        },
        null,
        2,
      ) + "\n",
    );
    console.log(
      `PASS: native ${phase}, exact migration backup, isolated memory, instructions and empty-memory gate`,
    );
  } finally {
    if (previous) {
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
    }
    await new Promise((done) => server.close(done));
    await browser.close();
  }
} else {
  throw new Error(
    "Usage: node scripts/verify-profile-memory-native.mjs [backend|ui|restart]",
  );
}
