import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 680, height: 570 });
  await page.addInitScript(() => {
    const run = (id: number, text: string, prompt: string | null) => ({
      id,
      entry_id: 1,
      session_id: null,
      started_at: "2026-09-27T10:00:00Z",
      ended_at: "2026-09-27T10:00:01Z",
      status: "succeeded",
      original_text: "original",
      processed_text: text,
      profile_id: null,
      profile_name: null,
      profile_revision: null,
      prompt_source: "selected_prompt",
      prompt_template: prompt,
      elapsed_ms: 1000,
      stop_to_output_ms: null,
      output_outcome: "history_only",
      error_code: null,
      error_detail: null,
      provider_id: "openai",
      provider_name: "OpenAI",
      requested_model: "gpt-6-luna",
      compatibility_cache_hit: false,
      validated_operation: null,
      validated_effect_kind: null,
      details_incomplete: false,
    });
    const call = (id: number, runId: number) => ({
      id,
      run_id: runId,
      ordinal: 1,
      purpose: "rewrite",
      retry_of: null,
      provider_id: "openai",
      provider_name: "OpenAI",
      endpoint_label: "api.openai.com/v1",
      requested_model: "gpt-6-luna",
      reported_model: "gpt-6-luna",
      started_at: "2026-09-27T10:00:00Z",
      ended_at: "2026-09-27T10:00:01Z",
      elapsed_ms: 900,
      http_status: 200,
      outcome: "succeeded",
      error_code: null,
      provider_request_id: null,
      usage_json: null,
      rate_json: null,
      cost_usd: null,
      archive_status: "retained",
      request_bytes: 40,
    });
    const runs = [
      run(101, "new rewrite", "new prompt"),
      run(100, "old rewrite", null),
    ];
    const calls: Record<number, unknown[]> = {
      101: [call(201, 101)],
      100: [call(200, 100)],
    };
    const state = window as unknown as {
      __history: {
        cleared: boolean;
        slowRun: boolean;
        slowCall: number | null;
        requestCount: number;
      };
      __TAURI_INTERNALS__: unknown;
    };
    state.__history = {
      cleared: false,
      slowRun: false,
      slowCall: null,
      requestCount: 0,
    };
    state.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (
        command: string,
        args: { runId?: number; callId?: number },
      ) => {
        if (command === "get_history_processing_runs")
          return structuredClone(runs);
        if (command === "get_history_processing_run") {
          const selected = runs.find((item) => item.id === args.runId);
          if (state.__history.slowRun)
            await new Promise((resolve) => setTimeout(resolve, 400));
          return selected
            ? {
                run: {
                  ...structuredClone(selected),
                  prompt_template: state.__history.cleared
                    ? null
                    : selected.prompt_template,
                },
                calls: structuredClone(calls[selected.id]),
              }
            : null;
        }
        if (command === "get_history_request_contents") {
          state.__history.requestCount++;
          if (args.callId === state.__history.slowCall)
            await new Promise((resolve) => setTimeout(resolve, 400));
          return {
            call_id: args.callId,
            archive_status: state.__history.cleared ? "cleared" : "retained",
            request_json: state.__history.cleared
              ? null
              : JSON.stringify({
                  model: "gpt-6-luna",
                  marker: `request-${args.callId}`,
                }),
            byte_count: 40,
            prompt_template: state.__history.cleared ? null : "prompt",
          };
        }
        return null;
      },
    };
  });
  await page.route("**/history-inspector-test", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0"><div id="root"></div><script type="module">
        import React from '/node_modules/.vite/deps/react.js';
        import ReactDOM from '/node_modules/.vite/deps/react-dom_client.js';
        import '/src/App.css';
        import '/src/i18n/index.ts';
        import RefreshRuntime from '/@react-refresh';
        RefreshRuntime.injectIntoGlobalHook(window);
        window.$RefreshReg$ = () => {};
        window.$RefreshSig$ = () => type => type;
        window.__vite_plugin_react_preamble_installed__ = true;
        const { HistoryInspector } = await import('/src/components/settings/history/HistoryInspector.tsx');
        const root = ReactDOM.createRoot(document.getElementById('root'));
        const entry = { id: 1, file_name: 'fixture.wav', timestamp: 0, saved: false,
          title: 'fixture', transcription_text: 'original', post_processed_text: 'latest entry text',
          post_process_prompt: 'legacy entry prompt', post_process_requested: true, processing_summary: null };
        let revision = 0;
        window.__refreshHistory = () => root.render(React.createElement(HistoryInspector,
          { entry, onClose: () => {}, refreshKey: String(++revision), archiveRevision: window.__history.cleared ? 1 : 0, metadataWarning: false }));
        window.__refreshHistory();
      </script></body></html>`,
    }),
  );
  await page.goto("/history-inspector-test");
  await expect(page.getByRole("tab", { name: "Overview" })).toBeVisible();
  await expect(page.getByText("new rewrite")).toBeVisible();
});

test("clear while viewing Overview invalidates a retained Requests tab", async ({
  page,
}) => {
  await page.getByRole("tab", { name: "Requests" }).click();
  await expect(page.getByText(/request-201/)).toBeVisible();
  await page.getByRole("tab", { name: "Overview" }).click();
  await page.evaluate(() => {
    const state = window as unknown as {
      __history: { cleared: boolean; slowRun: boolean };
      __refreshHistory: () => void;
    };
    state.__history.cleared = true;
    state.__history.slowRun = true;
    state.__refreshHistory();
  });
  await expect(page.getByText("new prompt")).toHaveCount(0);
  await page.getByRole("tab", { name: "Requests" }).click();
  await expect(
    page.getByText("Saved request contents were cleared."),
  ).toBeVisible();
  await expect(page.getByText(/request-201/)).toHaveCount(0);
});

test("older run shows only its own output and stale call contents never cross runs", async ({
  page,
}) => {
  await page.evaluate(() => {
    (
      window as unknown as { __history: { slowCall: number } }
    ).__history.slowCall = 201;
  });
  await page.getByRole("tab", { name: "Requests" }).click();
  await page.locator("select").selectOption("100");
  await expect(
    page.getByRole("button", { name: /Call 1/ }).first(),
  ).toBeVisible();
  await expect(page.getByText(/request-200/)).toBeVisible();
  await page.waitForTimeout(450);
  await expect(page.getByText(/request-201/)).toHaveCount(0);
  await page.getByRole("tab", { name: "Overview" }).click();
  await expect(page.getByText("old rewrite")).toBeVisible();
  await expect(page.getByText("latest entry text")).toHaveCount(0);
  await expect(page.getByText("legacy entry prompt")).toHaveCount(0);
  await expect(
    page.getByText(
      "Requires complete usage and a verified price for this model and endpoint.",
    ),
  ).toBeVisible();
});

test("tabs support keyboard navigation in compact layout", async ({ page }) => {
  await page.getByRole("tab", { name: "Overview" }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Requests" })).toBeFocused();
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: "Calls" })).toBeFocused();
  await expect(page.locator("body")).toHaveJSProperty("scrollWidth", 680);
});

test("wide inspector renders in light and dark themes without page errors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ width: 1200, height: 800 });
  const inspector = page.getByRole("region", {
    name: "Post-processing details",
  });
  const colors: string[] = [];
  for (const theme of ["light", "dark"]) {
    await page.evaluate((value) => {
      document.documentElement.dataset.theme = value;
    }, theme);
    await expect(inspector).toBeVisible();
    colors.push(
      await inspector.evaluate(
        (element) => getComputedStyle(element).backgroundColor,
      ),
    );
    await expect(page.locator("body")).toHaveJSProperty("scrollWidth", 1200);
  }
  expect(colors[0]).not.toBe(colors[1]);
  expect(errors).toEqual([]);
});
