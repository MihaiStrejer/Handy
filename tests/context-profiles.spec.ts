import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  page.on("pageerror", (error) => console.error(error.message));
  await page.setViewportSize({ width: 680, height: 570 });
  await page.addInitScript(() => {
    let catalog = {
      schema_version: 1,
      revision: 0,
      next_id: 3,
      profiles: [
        {
          id: "general",
          name: "General",
          revision: 0,
          dictionary_revision: 0,
          icon: "generic",
          rules: [],
          prompt: "Rewrite {{transcript}} using {{dictionary}}.",
          dictionary: [],
        },
        {
          id: "terminal",
          name: "Terminal",
          revision: 0,
          dictionary_revision: 0,
          icon: "terminal",
          rules: [{ application: "WindowsTerminal.exe", workspace: null }],
          prompt: null,
          dictionary: [
            { id: "codex", canonical: "Codex", misheard_forms: ["codecks"] },
          ],
        },
        {
          id: "mail",
          name: "Mail",
          revision: 0,
          dictionary_revision: 0,
          icon: "mail",
          rules: [{ application: "mail.exe", workspace: null }],
          prompt: null,
          dictionary: [],
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
          if (
            args.profile.dictionary.some((item: any) =>
              item.misheard_forms.includes("collision"),
            )
          )
            throw "Phrase already belongs to another keyword";
          const profile = structuredClone(args.profile);
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
      "Use BOM (bill of materials) when bomb refers to this term.",
    ),
  );
  await expect(
    page.getByText(
      "Use BOM (bill of materials) when bomb refers to this term.",
    ),
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

test("dictionary keeps canonical left, preserves rejected draft and isolates profiles", async ({
  page,
}) => {
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  const canonical = page.getByRole("textbox", { name: "Canonical keyword" });
  const forms = page.getByRole("textbox", { name: "Misheard forms" });
  expect((await canonical.boundingBox())!.x).toBeLessThan(
    (await forms.boundingBox())!.x,
  );
  await expect(canonical).toHaveValue("Codex");
  await forms.fill("codecks\ncollision");
  await expect(page.getByRole("alert")).toContainText("already belongs");
  await expect(forms).toHaveValue("codecks\ncollision");
  await forms.fill("codecks\ncode X");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.getByRole("button", { name: "Mail mail.exe" }).click();
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  await expect(page.getByText("No keywords yet.")).toBeVisible();
  await page
    .getByRole("button", { name: "Terminal WindowsTerminal.exe" })
    .click();
  await page.getByRole("tab", { name: "Long-term dictionary" }).click();
  await expect(forms).toHaveValue("codecks\ncode X");
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
    "Rewrite {{transcript}} using {{dictionary}}.",
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
            ...catalog.profiles[1],
            id: "",
            name: `${name} ${index + 1}`,
            dictionary: [],
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
      path: `design/proof/profiles-layout-${width}.png`,
    });
  });
}
