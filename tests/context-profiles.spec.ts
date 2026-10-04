import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  page.on("pageerror", (error) => console.error(error.message));
  await page.setViewportSize({ width: 680, height: 570 });
  await page.addInitScript(() => {
    let catalog = {
      schema_version: 3,
      revision: 0,
      next_id: 3,
      profiles: [
        {
          id: "general",
          name: "General",
          revision: 0,
          rewrite_context_revision: 0,
          long_term_memory_revision: 0,
          consolidation_instructions: null,
          long_term_undo: null,
          last_consolidation: null,
          icon: "generic",
          rules: [],
          prompt: "Rewrite {{transcript}} using {{long_term_memory}}.",
          long_term_memory: "",
        },
        {
          id: "terminal",
          name: "Terminal",
          revision: 0,
          rewrite_context_revision: 0,
          long_term_memory_revision: 0,
          consolidation_instructions: null,
          long_term_undo: null,
          last_consolidation: null,
          icon: "terminal",
          rules: [{ application: "WindowsTerminal.exe", workspace: null }],
          prompt: null,
          long_term_memory: "Use Codex for codecks.",
        },
        {
          id: "mail",
          name: "Mail",
          revision: 0,
          rewrite_context_revision: 0,
          long_term_memory_revision: 0,
          consolidation_instructions: null,
          long_term_undo: null,
          last_consolidation: null,
          icon: "mail",
          rules: [{ application: "mail.exe", workspace: null }],
          prompt: null,
          long_term_memory: "",
        },
      ],
    };
    const memory: Record<string, unknown[]> = {
      terminal: [
        {
          id: "1",
          text: "Use Thursday for the deadline.",
          source_session: "42",
        },
      ],
      mail: [
        { id: "2", text: "Keep the greeting brief.", source_session: "43" },
      ],
    };
    (window as any).__catalog = () => catalog;
    let nextCallback = 1;
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<number, { event: string; handler: number }>();
    let nextListener = 1;
    (window as any).__memoryEvent = (profile: string, text: string) => {
      memory[profile] = [{ id: "learned", text, source_session: "100" }];
      for (const [id, listener] of listeners) {
        if (listener.event === "profile-memory-updated")
          callbacks.get(listener.handler)?.({
            event: listener.event,
            id,
            payload: profile,
          });
      }
    };
    const operations: Record<string, any> = {};
    const emitOperation = (operation: any) => {
      for (const [id, listener] of listeners) {
        if (listener.event === "profile-consolidation-updated")
          callbacks.get(listener.handler)?.({
            event: listener.event,
            id,
            payload: operation,
          });
      }
    };
    (window as any).__resolveConsolidation = (
      profileId: string,
      status = "committed",
      text = "Prior terminology and new context",
    ) => {
      const operation = operations[profileId];
      if (!operation || operation.status !== "running") return;
      const profile = catalog.profiles.find((p) => p.id === profileId)!;
      operation.status = status;
      if (status === "committed") {
        profile.long_term_undo = {
          text: profile.long_term_memory,
          committed_revision: profile.long_term_memory_revision + 1,
        } as any;
        profile.long_term_memory = text;
        profile.long_term_memory_revision++;
        catalog.revision++;
        profile.revision = catalog.revision;
        operation.can_undo = true;
        profile.last_consolidation = structuredClone(operation);
      }
      emitOperation(operation);
    };
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: (callback: (event: unknown) => void) => {
        const id = nextCallback++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback: (id: number) => callbacks.delete(id),
      invoke: async (command: string, args: any) => {
        if (command === "plugin:event|listen") {
          const id = nextListener++;
          listeners.set(id, { event: args.event, handler: args.handler });
          return id;
        }
        if (command === "plugin:event|unlisten") {
          listeners.delete(args.eventId);
          return;
        }
        if (command === "plugin:dialog|open")
          return (window as any).__iconFile ?? null;
        if (command === "import_profile_icon") {
          if ((window as any).__iconError) throw "Invalid icon image";
          return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
        }
        if (command === "get_context_profiles") return structuredClone(catalog);
        if (command === "get_app_settings") return { app_language: "en" };
        if (command === "plugin:os|locale") return "en-US";
        if (command === "save_context_profile") {
          const state = window as any;
          state.__saveStarted = (state.__saveStarted ?? 0) + 1;
          (state.__saveProfiles ??= []).push(structuredClone(args.profile));
          state.__saveActive = (state.__saveActive ?? 0) + 1;
          state.__saveMaxActive = Math.max(
            state.__saveMaxActive ?? 0,
            state.__saveActive,
          );
          await new Promise((resolve) =>
            setTimeout(resolve, state.__saveDelay ?? 0),
          );
          state.__saveActive--;
          if (args.expectedRevision !== catalog.revision)
            throw "Profiles changed; reload before saving";
          if (!args.profile.name.trim())
            throw "Profile name must contain 1 to 80 characters";
          if (args.profile.prompt?.includes("{{unknown}}"))
            throw "Unknown prompt variable";
          if ("long_term_memory" in args.profile)
            throw "Memory is not editable";
          const existing = catalog.profiles.find(
            (p) => p.id === args.profile.id,
          );
          const profile = {
            revision: 0,
            rewrite_context_revision: 0,
            long_term_memory_revision: 0,
            long_term_memory: "",
            long_term_undo: null,
            last_consolidation: null,
            ...existing,
            ...structuredClone(args.profile),
          };
          profile.name = profile.name.trim();
          profile.id ||= `new-${catalog.next_id++}`;
          catalog = {
            ...catalog,
            revision: catalog.revision + 1,
            profiles: [
              ...catalog.profiles.filter((p) => p.id !== profile.id),
              profile,
            ],
          };
          return structuredClone(catalog);
        }
        if (command === "get_profile_consolidation")
          return structuredClone(
            operations[args.profileId] ??
              catalog.profiles.find((p) => p.id === args.profileId)
                ?.last_consolidation ??
              null,
          );
        if (command === "start_profile_consolidation") {
          const state = window as any;
          state.__consolidationStarts = (state.__consolidationStarts ?? 0) + 1;
          state.__consolidationInstructions = args.instructions;
          if (state.__consolidationError) throw state.__consolidationError;
          if (!memory[args.profileId]?.length) throw "empty_short_term";
          if (operations[args.profileId]?.status === "running")
            throw "already_running";
          const operation = {
            operation_id: `op-${state.__consolidationStarts}`,
            profile_id: args.profileId,
            provider_name: "Fixture provider",
            model: "fixture-model",
            status: "running",
            error_code: null,
            can_undo: false,
          };
          operations[args.profileId] = operation;
          emitOperation(operation);
          return structuredClone(operation);
        }
        if (command === "cancel_profile_consolidation") {
          const operation = operations[args.profileId];
          if (operation.operation_id !== args.operationId)
            throw "operation_changed";
          if (operation.status === "running") operation.status = "cancelled";
          emitOperation(operation);
          return structuredClone(operation);
        }
        if (command === "undo_profile_consolidation") {
          const profile = catalog.profiles.find(
            (p) => p.id === args.profileId,
          )!;
          if (profile.long_term_memory_revision !== args.expectedRevision)
            throw "source_changed";
          const undo = profile.long_term_undo as any;
          if (!undo) throw "undo_unavailable";
          profile.long_term_memory = undo.text;
          profile.long_term_memory_revision++;
          profile.long_term_undo = null;
          catalog.revision++;
          profile.revision = catalog.revision;
          const operation = operations[args.profileId];
          operation.status = "undone";
          operation.can_undo = false;
          profile.last_consolidation = structuredClone(operation);
          emitOperation(operation);
          return structuredClone(catalog);
        }
        if (command === "get_profile_memory")
          return structuredClone(memory[args.profileId] ?? []);
        if (command === "remove_profile_memory") {
          memory[args.profileId] = args.itemId
            ? memory[args.profileId].filter(
                (item: any) => item.id !== args.itemId,
              )
            : [];
          return;
        }
        return null;
      },
    };
  });
  await page.route("**/profile-test", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0"><div style="margin-left:150px;height:calc(100vh - 36px);display:flex;min-width:0;overflow:hidden" id="root"></div><script type="module">
    import React from '/node_modules/.vite/deps/react.js';
    import ReactDOM from '/node_modules/.vite/deps/react-dom_client.js';
    import '/src/App.css';
    import '/src/i18n/index.ts';
    import RefreshRuntime from '/@react-refresh';
    RefreshRuntime.injectIntoGlobalHook(window);
    window.$RefreshReg$ = () => {};
    window.$RefreshSig$ = () => type => type;
    window.__vite_plugin_react_preamble_installed__ = true;
    const { ContextProfilesSettings } = await import('/src/components/settings/context-profiles/ContextProfilesSettings.tsx');
    const root = ReactDOM.createRoot(document.getElementById('root'));
    let renderKey = 0;
    window.__remountProfiles = () => root.render(React.createElement(ContextProfilesSettings, {key: ++renderKey}));
    window.__hideProfiles = () => root.render(null);
    window.__remountProfiles();
  </script></body></html>`,
    }),
  );
  await page.goto("/profile-test");
  await expect(
    page.getByRole("navigation", { name: "Context profiles" }),
  ).toBeVisible();
});

test("accepted correction refreshes visible memory only for its profile", async ({
  page,
}) => {
  await page
    .getByRole("tab", { name: "Short-term memory", exact: true })
    .click();
  await expect(page.getByText("No short-term memory yet.")).toBeVisible();
  await page.evaluate(() =>
    (window as any).__memoryEvent(
      "general",
      "Use BOM when bomb refers to this term.",
    ),
  );
  await expect(
    page.getByText("Use BOM when bomb refers to this term."),
  ).toBeVisible();
  await page.evaluate(() =>
    (window as any).__memoryEvent("terminal", "Private terminal correction"),
  );
  await expect(page.getByText("Private terminal correction")).toHaveCount(0);
  await page.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(page.getByText("No short-term memory yet.")).toBeVisible();
});

test("inheritance, cancelled customization, invalid draft, custom save and reset", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await expect(
    page.getByText("Inherited from General.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  const prompt = page.getByRole("textbox", { name: "System prompt" });
  await prompt.fill("Uncommitted change");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Cancel", exact: true })
    .last()
    .click();
  await expect(
    page.getByText("Inherited from General.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  await prompt.fill("{{unknown}}");
  await page
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Unknown prompt variable",
  );
  await expect(prompt).toHaveValue("{{unknown}}");
  await prompt.fill("Custom ");
  await page
    .getByRole("button", { name: "{{input_context}}", exact: true })
    .click();
  await expect(prompt).toBeFocused();
  await expect(prompt).toHaveValue("Custom {{input_context}}");
  await page
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(
    page.getByText("Custom prompt for this profile.", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Edit prompt", exact: true }).click();
  await page
    .getByRole("button", { name: "Use General prompt", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(
    page.getByText("Inherited from General.", { exact: false }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await page.getByRole("button", { name: "Edit prompt", exact: true }).click();
  await prompt.fill("Updated General");
  await page
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await expect(page.locator("pre")).toHaveText("Updated General");
});

test("long-term text is read-only, isolated, and excluded from metadata autosave", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(page.getByText("Use Codex for codecks.")).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Consolidation instructions" }),
  ).toHaveCount(1);
  await page.getByRole("tab", { name: "Context selection" }).click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("Terminal renamed");
  await page.getByRole("button", { name: "Mail mail.exe" }).click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(page.getByText("No long-term memory yet.")).toBeVisible();
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((p: any) => p.id === "terminal").long_term_memory,
    ),
  ).toBe("Use Codex for codecks.");
});

test("memory removal and clear stay profile-local; tabs support keyboard navigation", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.getByRole("tab", { name: "Context selection" }).focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("tab", { name: "Short-term memory" }),
  ).toBeFocused();
  await expect(page.getByText("Use Thursday for the deadline.")).toBeVisible();
  await page.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(page.getByText("No short-term memory yet.")).toBeVisible();
  await page.getByRole("button", { name: "Mail mail.exe" }).click();
  await page.getByRole("tab", { name: "Short-term memory" }).click();
  await expect(page.getByText("Keep the greeting brief.")).toBeVisible();
  await page.getByRole("button", { name: "Clear memory", exact: true }).click();
  await expect(page.getByText("No short-term memory yet.")).toBeVisible();
});

test("adds a profile and fits the minimum window width with long content", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "+ Add profile", exact: true })
    .click();
  const name = "A long workspace profile name ".repeat(2);
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill(name);
  await expect(
    page
      .getByRole("navigation")
      .getByRole("button", { name: `${name.trim()} WindowsTerminal.exe` }),
  ).toBeVisible();
  await expect(page.getByRole("tab")).toHaveCount(3);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(680);
});

test("project template saves a directory without an executable and keeps existing memory", async ({
  page,
}) => {
  await page
    .getByRole("combobox", { name: "Profile template" })
    .selectOption("project");
  await page
    .getByRole("button", { name: "+ Add profile", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("Shared project");
  await page
    .getByRole("textbox", { name: "Working directory (primary)", exact: true })
    .fill("D:\\Work\\Project");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any)
            .__catalog()
            .profiles.find((p: any) => p.name === "Shared project")?.rules,
      ),
    )
    .toEqual([{ application: "", workspace: "D:\\Work\\Project" }]);
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((p: any) => p.id === "terminal").long_term_memory,
    ),
  ).toBe("Use Codex for codecks.");
});

test("T3 template includes stable and nightly fallback rules", async ({
  page,
}) => {
  await page
    .getByRole("combobox", { name: "Profile template" })
    .selectOption("t3");
  await page
    .getByRole("button", { name: "+ Add profile", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("T3 projects");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any)
            .__catalog()
            .profiles.find((p: any) => p.name === "T3 projects")?.rules,
      ),
    )
    .toEqual([
      { application: "T3 Code.exe", workspace: null },
      { application: "T3 Code (Nightly).exe", workspace: null },
    ]);
});

test("icon choices support keyboard selection and save the chosen icon", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  const icons = page.getByRole("radiogroup", { name: "Profile icon" });
  await expect(icons.getByRole("radio")).toHaveCount(15);
  await expect(
    page.getByRole("combobox", { name: "Profile icon" }),
  ).toHaveCount(0);
  const terminal = icons.getByRole("radio", { name: "Terminal", exact: true });
  await expect(terminal).toBeChecked();
  await terminal.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    icons.getByRole("radio", { name: "Code", exact: true }),
  ).toBeFocused();
  await expect(
    icons.getByRole("radio", { name: "Code", exact: true }),
  ).toBeChecked();
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await expect(
    icons.getByRole("radio", { name: "Code", exact: true }),
  ).toBeChecked();
});

test("imported icons handle cancellation and errors, persist and switch back to built-ins", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  const icons = page.getByRole("radiogroup", { name: "Profile icon" });
  const upload = page.getByRole("button", {
    name: "Import PNG or ICO…",
    exact: true,
  });
  await upload.click();
  await expect(
    icons.getByRole("radio", { name: "Terminal", exact: true }),
  ).toBeChecked();
  await page.evaluate(() => {
    (window as any).__iconFile = "C:/fixture/icon.png";
    (window as any).__iconError = true;
  });
  await upload.click();
  await expect(page.getByRole("alert")).toContainText("Invalid icon image");
  await expect(
    icons.getByRole("radio", { name: "Terminal", exact: true }),
  ).toBeChecked();
  await page.evaluate(() => {
    (window as any).__iconError = false;
  });
  await upload.click();
  await expect(
    icons.getByRole("radio", { name: "Custom", exact: true }),
  ).toBeChecked();
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await expect(
    icons.getByRole("radio", { name: "Custom", exact: true }),
  ).toBeChecked();
  const image = page
    .getByRole("navigation", { name: "Context profiles" })
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .locator("img");
  await expect(image).toHaveAttribute("src", /^data:image\/png;base64,/);
  await expect
    .poll(() =>
      image.evaluate(
        (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
      ),
    )
    .toBe(true);
  await icons.getByRole("radio", { name: "Chrome", exact: true }).check();
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await expect(
    icons.getByRole("radio", { name: "Chrome", exact: true }),
  ).toBeChecked();
});

test("autosave serializes slow responses and keeps newer typing", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.evaluate(() => {
    (window as any).__saveDelay = 700;
  });
  const name = page.getByRole("textbox", { name: "Profile name", exact: true });
  await name.fill("First name");
  await expect
    .poll(() => page.evaluate(() => (window as any).__saveStarted ?? 0))
    .toBe(1);
  await expect(name).toBeEnabled();
  await name.fill("Newest name");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any)
            .__catalog()
            .profiles.find((profile: any) => profile.id === "terminal").name,
      ),
    )
    .toBe("Newest name");
  await expect(name).toHaveValue("Newest name");
  expect(await page.evaluate(() => (window as any).__saveMaxActive)).toBe(1);
  await expect(
    page.getByRole("button", { name: "Save profile", exact: true }),
  ).toHaveCount(0);
});

test("switching profiles flushes pending edits and retains a rejected draft", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  const name = page.getByRole("textbox", { name: "Profile name", exact: true });
  await name.fill("Workspace renamed");
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await expect(
    page.getByRole("textbox", { name: "Profile name", exact: true }),
  ).toHaveValue("General");
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((profile: any) => profile.id === "terminal").name,
    ),
  ).toBe("Workspace renamed");
  await page
    .getByRole("button", { name: "Workspace renamed WindowsTerminal.exe" })
    .click();
  await name.fill("");
  await page
    .getByRole("button", { name: "General Fallback", exact: true })
    .click();
  await expect(name).toHaveValue("");
  await expect(page.getByRole("alert")).toBeVisible();
  await name.fill("Recovered workspace");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any)
            .__catalog()
            .profiles.find((profile: any) => profile.id === "terminal").name,
      ),
    )
    .toBe("Recovered workspace");
});

test("a late rejected save does not strand the corrected draft", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.evaluate(() => {
    (window as any).__saveDelay = 900;
  });
  const name = page.getByRole("textbox", { name: "Profile name", exact: true });
  await name.fill("");
  await expect
    .poll(() => page.evaluate(() => (window as any).__saveStarted ?? 0))
    .toBe(1);
  await name.fill("Corrected while saving");
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any)
            .__catalog()
            .profiles.find((profile: any) => profile.id === "terminal").name,
      ),
    )
    .toBe("Corrected while saving");
  await expect(name).toHaveValue("Corrected while saving");
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("leaving and reopening profiles waits for the pending autosave", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.evaluate(() => {
    (window as any).__saveDelay = 700;
  });
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("Saved across navigation");
  await page.evaluate(() => (window as any).__hideProfiles());
  await expect(
    page.getByRole("navigation", { name: "Context profiles" }),
  ).toHaveCount(0);
  await page.evaluate(() => (window as any).__remountProfiles());
  await expect(
    page.getByRole("button", {
      name: "Saved across navigation WindowsTerminal.exe",
    }),
  ).toBeVisible();
  expect(await page.evaluate(() => (window as any).__saveMaxActive)).toBe(1);
});

test("prompt modal stages edits until save and discards Cancel and Escape", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  const modal = page.getByRole("dialog", {
    name: "System prompt",
    exact: true,
  });
  const prompt = modal.getByRole("textbox", {
    name: "System prompt",
    exact: true,
  });
  await expect(prompt).toBeFocused();
  await prompt.fill("Draft that must not autosave");
  await page.waitForTimeout(650);
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((profile: any) => profile.id === "terminal").prompt,
    ),
  ).toBeNull();
  await page.keyboard.press("Escape");
  await expect(modal).toHaveCount(0);
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  await expect(prompt).toHaveValue(
    "Rewrite {{transcript}} using {{long_term_memory}}.",
  );
  await prompt.fill("Another discarded draft");
  await modal
    .getByRole("button", { name: "Cancel", exact: true })
    .last()
    .click();
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((profile: any) => profile.id === "terminal").prompt,
    ),
  ).toBeNull();
  await page.getByRole("button", { name: "Customize", exact: true }).click();
  await prompt.fill("Committed {{transcript}}");
  await modal
    .getByRole("button", { name: "Save and close", exact: true })
    .click();
  await expect(modal).toHaveCount(0);
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((profile: any) => profile.id === "terminal").prompt,
    ),
  ).toBe("Committed {{transcript}}");
  expect(await page.evaluate(() => (window as any).__saveStarted)).toBe(1);
});

for (const width of [680, 1050]) {
  test(`many long profile names scroll independently with a full-height divider at ${width}px`, async ({
    page,
  }) => {
    const height = width === 680 ? 570 : 760;
    await page.setViewportSize({ width, height });
    await page.emulateMedia({ colorScheme: width === 680 ? "light" : "dark" });
    const longName =
      "Workspace with a deliberately long profile name for a large project";
    await page.evaluate(async (name) => {
      const invoke = (window as any).__TAURI_INTERNALS__.invoke;
      for (let index = 0; index < 24; index++) {
        const catalog = await invoke("get_context_profiles");
        await invoke("save_context_profile", {
          expectedRevision: catalog.revision,
          profile: {
            icon: catalog.profiles[1].icon,
            rules: catalog.profiles[1].rules,
            prompt: catalog.profiles[1].prompt,
            id: "",
            name: `${name} ${index + 1}`,
            consolidation_instructions: null,
          },
        });
      }
      // Remount with the expanded fixture catalog, without navigating/reseeding it.
      (window as any).__remountProfiles();
    }, longName);
    const sidebar = page.getByRole("navigation", { name: "Context profiles" });
    const list = page.locator(".profile-list");
    const add = sidebar.getByRole("button", {
      name: "+ Add profile",
      exact: true,
    });
    await expect(
      sidebar.getByRole("button", {
        name: `${longName} 24 WindowsTerminal.exe`,
        exact: true,
      }),
    ).toHaveCount(1);
    const originalAdd = await add.boundingBox();
    const bounds = await page.locator(".context-profiles").boundingBox();
    const sidebarBounds = await sidebar.boundingBox();
    expect(
      Math.abs(sidebarBounds!.height - bounds!.height),
    ).toBeLessThanOrEqual(1);
    expect(sidebarBounds!.height).toBeGreaterThan(height - 60);
    expect(
      await list.evaluate(
        (element) => element.scrollHeight > element.clientHeight,
      ),
    ).toBe(true);
    await list.evaluate((element) => {
      element.scrollTop = element.scrollHeight;
    });
    const last = sidebar.getByRole("button", {
      name: `${longName} 24 WindowsTerminal.exe`,
      exact: true,
    });
    await expect(last).toBeInViewport();
    await expect(last).toHaveAttribute("title", `${longName} 24`);
    await last.click();
    await expect(
      page.getByRole("textbox", { name: "Profile name", exact: true }),
    ).toHaveValue(`${longName} 24`);
    expect((await add.boundingBox())!.y).toBe(originalAdd!.y);
    await expect(add).toBeInViewport();
    const sidebarScroll = await list.evaluate((element) => element.scrollTop);
    await page.locator(".profile-detail").evaluate((element) => {
      element.scrollTop = element.scrollHeight;
    });
    expect(await list.evaluate((element) => element.scrollTop)).toBe(
      sidebarScroll,
    );
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(width);
    await page.locator(".profile-detail").evaluate((element) => {
      element.scrollTop = 0;
    });
    await page.screenshot({
      path: `design/proof/profile-memory/profiles-layout-${width}.png`,
    });
  });
}

test("consolidation flushes instructions, keeps notes, survives navigation and undoes only long-term text", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  const instructions = page.getByRole("textbox", {
    name: "Consolidation instructions",
  });
  const update = page.getByRole("button", {
    name: "Update long-term memory",
    exact: true,
  });
  expect((await instructions.boundingBox())!.x).toBeLessThan(
    (await update.boundingBox())!.x,
  );
  await instructions.fill("Keep technical terms and translation context");
  await update.click();
  await expect(page.getByText("Updating long-term memory...")).toBeVisible();
  expect(
    await page.evaluate(() => (window as any).__consolidationInstructions),
  ).toBe("Keep technical terms and translation context");
  await page.getByRole("button", { name: "Mail mail.exe" }).click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page.evaluate(() =>
    (window as any).__resolveConsolidation(
      "terminal",
      "committed",
      "Use Codex for codecks.\nTranslation context",
    ),
  );
  await expect(
    page.getByText("Translation context", { exact: false }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(
    page.getByText("Use Codex for codecks.\nTranslation context"),
  ).toBeVisible();
  await expect(
    page.getByText("Fixture provider / fixture-model", { exact: false }),
  ).toBeVisible();
  await instructions.fill("Later instructions");
  await page.getByRole("button", { name: "Undo last update" }).click();
  await expect(page.getByText("Use Codex for codecks.")).toBeVisible();
  await expect(instructions).toHaveValue("Later instructions");
  await page
    .getByRole("tab", { name: "Short-term memory", exact: true })
    .click();
  await expect(page.getByText("Use Thursday for the deadline.")).toBeVisible();
});

test("cancelled consolidation cannot replace memory and status survives remount", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Update long-term memory", exact: true })
    .click();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByText("Update cancelled.")).toBeVisible();
  await page.evaluate(() =>
    (window as any).__resolveConsolidation(
      "terminal",
      "committed",
      "Late result",
    ),
  );
  await expect(page.getByText("Late result")).toHaveCount(0);
  await page.evaluate(() => (window as any).__remountProfiles());
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(page.getByText("Update cancelled.")).toBeVisible();
  await expect(page.getByText("Use Codex for codecks.")).toBeVisible();
});

test("rejected edit prevents consolidation, empty notes and missing model are distinct", async ({
  page,
}) => {
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Update long-term memory", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText(
      "Add a verified short-term correction before updating long-term memory.",
    ),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("");
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Update long-term memory", exact: true })
    .click();
  expect(
    await page.evaluate(() => (window as any).__consolidationStarts ?? 0),
  ).toBe(0);
  await page.getByRole("tab", { name: "Context selection" }).click();
  await page
    .getByRole("textbox", { name: "Profile name", exact: true })
    .fill("Terminal");
  await page.evaluate(() => {
    (window as any).__consolidationError = "missing_model";
  });
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Update long-term memory", exact: true })
    .click();
  await expect(
    page.getByText("Choose a post-processing model before updating memory."),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as any).__consolidationError = null;
  });
  await page.getByRole("button", { name: "Retry update" }).click();
  await expect(page.getByText("Updating long-term memory...")).toBeVisible();
});

test("late promotion cannot be overwritten by a metadata autosave", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Update long-term memory", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Consolidation instructions" })
    .fill("Newer draft");
  await page.evaluate(() =>
    (window as any).__resolveConsolidation(
      "terminal",
      "committed",
      "Preserved promotion",
    ),
  );
  await expect(page.getByText("Preserved promotion")).toBeVisible();
  await page.getByRole("button", { name: "Mail mail.exe" }).click();
  await expect(
    page.getByRole("textbox", { name: "Profile name", exact: true }),
  ).toHaveValue("Mail");
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((p: any) => p.id === "terminal").long_term_memory,
    ),
  ).toBe("Preserved promotion");
  expect(
    await page.evaluate(
      () =>
        (window as any)
          .__catalog()
          .profiles.find((p: any) => p.id === "terminal")
          .consolidation_instructions,
    ),
  ).toBe("Newer draft");
});

test("unmount autosave retries the changed catalog and preserves the promotion", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Update long-term memory", exact: true })
    .click();
  await expect(page.getByText("Updating long-term memory...")).toBeVisible();
  await page.evaluate(() => {
    (window as any).__saveDelay = 100;
  });
  await page
    .getByRole("textbox", { name: "Consolidation instructions" })
    .fill("Draft retained on unmount");
  await page.evaluate(() => {
    (window as any).__resolveConsolidation(
      "terminal",
      "committed",
      "Committed during unmount",
    );
    (window as any).__hideProfiles();
    (window as any).__remountProfiles();
  });
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await expect(page.getByText("Committed during unmount")).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Consolidation instructions" }),
  ).toHaveValue("Draft retained on unmount");
});

test("instruction and action row fits long translations at minimum width", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page
    .getByRole("tab", { name: "Long-term memory", exact: true })
    .click();
  await page.evaluate(async () => {
    // Runtime translation fixture, without changing the user's app settings.
    const { default: i18n } = await import("/src/i18n/index.ts");
    i18n.addResourceBundle(
      "memory-fixture",
      "translation",
      {
        profiles: {
          consolidation: {
            update:
              "Update long-term terminology and contextual translation memory now",
            instructions:
              "Instructions for bringing durable short-term knowledge into long-term memory",
          },
        },
      },
      true,
      true,
    );
    await i18n.changeLanguage("memory-fixture");
  });
  const action = page.getByRole("button", {
    name: "Update long-term terminology and contextual translation memory now",
  });
  await expect(action).toBeVisible();
  const input = page.getByRole("textbox", {
    name: "Instructions for bringing durable short-term knowledge into long-term memory",
  });
  expect((await input.boundingBox())!.x).toBeLessThan(
    (await action.boundingBox())!.x,
  );
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(680);
});
