// Native proof: existing ASR -> local HTTP fixture -> SQLite -> History UI.
// Run with Node while the isolated context-dev app exposes CDP on port 9223.
import { chromium, expect } from "@playwright/test";
import { createServer } from "node:http";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const dataDir = resolve("src-tauri/target/debug/Data");
const proofDir = resolve("design/proof/history-native");
const sourceWav = process.env.HANDY_HISTORY_PROOF_WAV;
if (!sourceWav)
  throw new Error("Set HANDY_HISTORY_PROOF_WAV to a synthetic speech WAV");
const fixtureName = `history-proof-${randomUUID()}.wav`;
let mode = "success";
const received = [];
const server = createServer(async (request, response) => {
  if (request.url !== "/v1/chat/completions") {
    request.resume();
    response.writeHead(404).end();
    return;
  }
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  received.push(JSON.parse(Buffer.concat(chunks).toString("utf8")));
  response.setHeader("Content-Type", "application/json");
  response.setHeader("x-request-id", "synthetic-history-proof");
  if (mode === "failure") {
    response.writeHead(429).end(
      JSON.stringify({
        error: {
          code: "rate_limit_exceeded",
          message: "SECRET-ECHO-MUST-NOT-BE-STORED",
        },
      }),
    );
  } else {
    response.end(
      JSON.stringify({
        model: "history-proof-returned",
        service_tier: "default",
        choices:
          mode === "invalid"
            ? "PRIVATE-MALFORMED-REPLY"
            : [{ message: { content: "History processing details work." } }],
        usage: {
          prompt_tokens: 120,
          completion_tokens: 8,
          total_tokens: 128,
          prompt_tokens_details: { cached_tokens: 64, cache_write_tokens: 0 },
          completion_tokens_details: { reasoning_tokens: 0 },
        },
      }),
    );
  }
});
await new Promise((done) => server.listen(0, "127.0.0.1", done));
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser
  .contexts()
  .flatMap((context) => context.pages())
  .find((page) => page.url() === "http://localhost:1420/");
if (!page) throw new Error("Native development WebView not found");
const invoke = (command, args) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
let previous;
let prompt;
let entryId;
const evidence = {
  checked_at: new Date().toISOString(),
  native: true,
  endpoint: "local fixture",
  checks: [],
};
try {
  expect((await invoke("get_app_dir_path")).toLowerCase()).toBe(
    dataDir.toLowerCase(),
  );
  expect(await invoke("is_recording")).toBe(false);
  previous = await invoke("get_app_settings");
  await invoke("change_post_process_profiles_setting", { enabled: false });
  await invoke("set_post_process_provider", { providerId: "custom" });
  await invoke("change_post_process_base_url_setting", {
    providerId: "custom",
    baseUrl: `http://127.0.0.1:${server.address().port}/v1`,
  });
  await invoke("change_post_process_model_setting", {
    providerId: "custom",
    model: "history-proof-requested",
  });
  await invoke("change_post_process_api_key_setting", {
    providerId: "custom",
    apiKey: "",
  });
  await invoke("change_post_process_enabled_setting", { enabled: true });
  await invoke("set_history_request_contents_enabled", { enabled: true });
  prompt = await invoke("add_post_process_prompt", {
    name: "Temporary native history proof",
    prompt: "Rewrite this synthetic transcript: ${output}",
  });
  await invoke("set_post_process_selected_prompt", { id: prompt.id });
  await copyFile(sourceWav, join(dataDir, "recordings", fixtureName));
  const insert = spawnSync(
    "uv",
    [
      "run",
      "python",
      "-c",
      `
import json,sqlite3,sys,time
args=json.load(sys.stdin)
with sqlite3.connect(args['database']) as db:
    cursor=db.execute('INSERT INTO transcription_history(file_name,timestamp,saved,title,transcription_text,post_process_requested) VALUES(?,?,1,?,?,1)',(args['file'],int(time.time()),'Native history proof','Synthetic recording awaiting retranscription'))
    print(cursor.lastrowid)
`,
    ],
    {
      input: JSON.stringify({
        database: join(dataDir, "history.db"),
        file: fixtureName,
      }),
      encoding: "utf8",
    },
  );
  if (insert.status !== 0) throw new Error(insert.stderr);
  entryId = Number(insert.stdout.trim());
  await page.reload();
  await invoke("show_main_window_command");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await invoke("retry_history_entry_transcription", { id: entryId });
  let runs = await invoke("get_history_processing_runs", { entryId });
  expect(runs).toHaveLength(1);
  expect(runs[0].status).toBe("succeeded");
  expect(runs[0].original_text.toLowerCase()).toContain("history");
  const success = await invoke("get_history_processing_run", {
    runId: runs[0].id,
  });
  expect(success.calls).toHaveLength(1);
  expect(success.calls[0].reported_model).toBe("history-proof-returned");
  expect(JSON.parse(success.calls[0].usage_json).total_tokens).toBe(128);
  expect(success.calls[0].cost_usd).toBeNull();
  const archived = await invoke("get_history_request_contents", {
    callId: success.calls[0].id,
  });
  expect(JSON.parse(archived.request_json)).toEqual(received[0]);
  await expect(
    page.getByText("Post-processed", { exact: true }).first(),
  ).toBeVisible();
  evidence.checks.push(
    "Native ASR, successful HTTP call, nullable custom-provider cost, and exact request archive",
  );

  mode = "failure";
  await invoke("retry_history_entry_transcription", { id: entryId });
  runs = await invoke("get_history_processing_runs", { entryId });
  expect(runs).toHaveLength(2);
  expect(runs[0].status).toBe("failed");
  expect(runs[1].status).toBe("succeeded");
  const failure = await invoke("get_history_processing_run", {
    runId: runs[0].id,
  });
  expect(failure.calls[0].http_status).toBe(429);
  expect(failure.calls[0].error_code).toBe("rate_limit");
  expect(failure.calls[0].usage_json).toBeNull();
  expect(JSON.stringify(failure)).not.toContain("SECRET-ECHO");
  evidence.checks.push(
    "Retry appends a failed run, preserving success; rejection prose excluded and unknown usage stays null",
  );

  await expect(
    page.getByText("Post-process failed", { exact: true }).first(),
  ).toBeVisible();
  await expect(
    page
      .getByText("The provider rate limit was reached.", { exact: true })
      .first(),
  ).toBeVisible();
  await mkdir(proofDir, { recursive: true });
  await page.screenshot({ path: join(proofDir, "failure-entry.png") });
  evidence.checks.push(
    "Native History updates without reload, rendering failure status and inline reason",
  );
  const inspectorTrigger = page
    .getByRole("button", { name: "Inspect post-processing", exact: true })
    .first();
  await inspectorTrigger.click();
  const inspector = page.getByRole("region", {
    name: "Post-processing details",
    exact: true,
  });
  await expect(inspector).toBeVisible();
  await inspector.getByRole("tab", { name: "Calls", exact: true }).click();
  await expect(inspector.getByText("429", { exact: true })).toBeVisible();
  await inspector
    .getByLabel("Attempt", { exact: true })
    .selectOption(String(success.run.id));
  await expect(inspector.getByText("200", { exact: true })).toBeVisible();
  await inspector.getByRole("tab", { name: "Requests", exact: true }).click();
  await expect(
    inspector.getByText("Exact sent request JSON", { exact: true }),
  ).toBeVisible();
  await expect(inspector.locator("pre")).toContainText(
    "history-proof-requested",
  );
  expect(received).toHaveLength(2);
  await page.screenshot({ path: join(proofDir, "request-inspector.png") });
  await inspector
    .getByRole("tab", { name: "Requests", exact: true })
    .press("Escape");
  await expect(inspector).not.toBeVisible();
  await expect(inspectorTrigger).toBeFocused();
  evidence.checks.push(
    "Native inspector switches immutable attempts, shows exact request and HTTP status without new requests, and restores focus on Escape",
  );
  mode = "success";
  await invoke("set_history_request_contents_enabled", { enabled: false });
  await invoke("retry_history_entry_transcription", { id: entryId });
  runs = await invoke("get_history_processing_runs", { entryId });
  expect(runs).toHaveLength(3);
  expect(runs[0].prompt_template).toBeNull();
  const disabledRun = await invoke("get_history_processing_run", {
    runId: runs[0].id,
  });
  expect(disabledRun.calls[0].usage_json).not.toBeNull();
  const disabledRequest = await invoke("get_history_request_contents", {
    callId: disabledRun.calls[0].id,
  });
  expect(disabledRequest.archive_status).toBe("disabled");
  expect(disabledRequest.request_json).toBeNull();
  const pageData = await invoke("get_history_entries", {
    cursor: null,
    limit: 30,
  });
  expect(
    pageData.entries.find((entry) => entry.id === entryId).post_process_prompt,
  ).toBeNull();
  evidence.checks.push(
    "Archive-off retains usage and output, excluding request JSON and both template fields",
  );
  mode = "invalid";
  await invoke("retry_history_entry_transcription", { id: entryId });
  runs = await invoke("get_history_processing_runs", { entryId });
  expect(runs).toHaveLength(4);
  expect(runs[0].status).toBe("failed");
  const invalidRun = await invoke("get_history_processing_run", {
    runId: runs[0].id,
  });
  expect(invalidRun.calls[0].http_status).toBe(200);
  expect(invalidRun.calls[0].error_code).toBe("invalid_response");
  expect(JSON.parse(invalidRun.calls[0].usage_json).total_tokens).toBe(128);
  expect(JSON.stringify(invalidRun)).not.toContain("PRIVATE-MALFORMED-REPLY");
  evidence.checks.push(
    "Malformed HTTP-200 response retains reported usage without archiving response prose",
  );
  evidence.http_attempts = received.length;
} finally {
  const restoreErrors = [];
  const restore = async (command, args) => {
    try {
      await invoke(command, args);
    } catch {
      restoreErrors.push(command);
    }
  };
  try {
    if (entryId) await restore("delete_history_entry", { id: entryId });
    if (previous) {
      if (prompt)
        await restore("delete_post_process_prompt", { id: prompt.id });
      await restore("change_post_process_profiles_setting", { enabled: false });
      await restore("change_post_process_base_url_setting", {
        providerId: "custom",
        baseUrl: previous.post_process_providers.find(
          (provider) => provider.id === "custom",
        ).base_url,
      });
      await restore("change_post_process_model_setting", {
        providerId: "custom",
        model: previous.post_process_models.custom ?? "",
      });
      await restore("change_post_process_api_key_setting", {
        providerId: "custom",
        apiKey: previous.post_process_api_keys.custom ?? "",
      });
      await restore("set_post_process_provider", {
        providerId: previous.post_process_provider_id,
      });
      await restore("change_post_process_enabled_setting", {
        enabled: previous.post_process_enabled,
      });
      await restore("change_post_process_profiles_setting", {
        enabled: previous.post_process_profiles,
      });
      await restore("set_history_request_contents_enabled", {
        enabled: previous.save_history_request_contents,
      });
      // The public selection command cannot clear a selection; restore only this
      // field through the same loaded store, preserving all other current settings.
      await page
        .evaluate(
          async ({ path, selected }) => {
            const { load } = await import(
              "/node_modules/@tauri-apps/plugin-store/dist-js/index.js"
            );
            const store = await load(path);
            const current = await store.get("settings");
            current.post_process_selected_prompt_id = selected;
            await store.set("settings", current);
            await store.save();
          },
          {
            path: join(dataDir, "settings_store.json"),
            selected: previous.post_process_selected_prompt_id,
          },
        )
        .catch(() => restoreErrors.push("restore selected prompt"));
      const restored = await invoke("get_app_settings");
      for (const field of [
        "post_process_provider_id",
        "post_process_enabled",
        "post_process_profiles",
        "save_history_request_contents",
        "post_process_selected_prompt_id",
      ]) {
        if (restored[field] !== previous[field])
          restoreErrors.push(`restore ${field}`);
      }
      if (
        restored.post_process_providers.find(
          (provider) => provider.id === "custom",
        ).base_url !==
        previous.post_process_providers.find(
          (provider) => provider.id === "custom",
        ).base_url
      )
        restoreErrors.push("restore custom endpoint");
      if (
        (restored.post_process_models.custom ?? "") !==
        (previous.post_process_models.custom ?? "")
      )
        restoreErrors.push("restore custom model");
      if (
        (restored.post_process_api_keys.custom ?? "") !==
        (previous.post_process_api_keys.custom ?? "")
      )
        restoreErrors.push("restore custom key");
      await page.reload();
    }
  } finally {
    await browser.close();
    server.close();
  }
  if (restoreErrors.length)
    throw new Error(`Cleanup failed: ${restoreErrors.join(", ")}`);
}
evidence.checks.push(
  "Fixture removed and prior provider, model, key, prompt, and feature settings restored",
);
await writeFile(
  join(proofDir, "result.json"),
  JSON.stringify(evidence, null, 2) + "\n",
);
console.log(JSON.stringify(evidence, null, 2));
