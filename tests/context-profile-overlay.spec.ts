import { test, expect, type Page } from "@playwright/test";

async function emit(page: Page, event: string, payload: unknown) {
  await page.evaluate(
    ({ event, payload }) => {
      (window as any).__emit(event, payload);
    },
    { event, payload },
  );
}

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    let serial = 0;
    const callbacks = new Map<number, Function>();
    const listeners = new Map<number, { event: string; handler: number }>();
    (window as any).__TAURI_INTERNALS__ = {
      transformCallback: (callback: Function) => {
        callbacks.set(++serial, callback);
        return serial;
      },
      unregisterCallback: (id: number) => callbacks.delete(id),
      invoke: async (command: string, args: any) => {
        if (command === "plugin:event|listen") {
          listeners.set(++serial, args);
          return serial;
        }
        if (command === "plugin:event|unlisten") {
          listeners.delete(args.eventId);
          return;
        }
        if (command === "get_app_settings")
          return {
            app_language: "en",
            overlay_position: (window as any).__position ?? "bottom",
            theme: "dark",
          };
        if (command === "plugin:os|locale") return "en-US";
        return null;
      },
    };
    (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: () => {},
    };
    (window as any).__emit = (event: string, payload: unknown) => {
      for (const [id, listener] of listeners) {
        if (listener.event === event)
          callbacks.get(listener.handler)?.({ event, id, payload });
      }
    };
    (window as any).__hasListener = (event: string) =>
      [...listeners.values()].some((item) => item.event === event);
  });
  await page.goto("/src/overlay/index.html");
  await page.waitForFunction(() =>
    (window as any).__hasListener("session-display"),
  );
});

const context = (session_id: string, profile: unknown = null) => ({
  session_id,
  enabled: true,
  profile,
});
const terminal = {
  name: "Handy workspace",
  icon: "terminal",
  input_mode: "edit",
};

test("profile replaces the dot, preserves microphone readiness, rejects stale events", async ({
  page,
}) => {
  await emit(page, "show-overlay", {
    state: "recording",
    context: context("1"),
  });
  await expect(page.locator(".sdot.arming")).toBeVisible();
  await emit(page, "session-display", context("1", terminal));
  await expect(
    page.getByRole("img", { name: "Handy workspace: Edit selection" }),
  ).toBeVisible();
  await expect(page.locator(".sdot")).toHaveCount(0);
  await expect(page.locator(".sprofile.arming")).toBeVisible();
  await expect(page.locator(".sprofile-edit")).toBeVisible();
  await emit(page, "recording-ready", null);
  await expect(page.locator(".sprofile.ready")).toBeVisible();
  await emit(page, "show-overlay", {
    state: "recording",
    context: context("2"),
  });
  await emit(page, "session-display", context("1", terminal));
  await expect(page.locator(".sdot")).toBeVisible();
  await emit(
    page,
    "session-display",
    context("2", { name: "General", icon: "missing", input_mode: "unknown" }),
  );
  await expect(
    page.getByRole("img", { name: "General: Selection unavailable" }),
  ).toBeVisible();
  await expect(page.locator(".sprofile-edit")).toHaveCount(0);
  await emit(page, "hide-overlay", null);
  await emit(page, "session-display", context("2", terminal));
  await expect(page.locator(".scard")).toHaveCount(0);
  await emit(page, "show-overlay", {
    state: "recording",
    context: { ...context("3"), enabled: false },
  });
  await expect(page.locator(".sdot")).toBeVisible();
});

test("bundled app and imported icons render in the recording slot", async ({
  page,
}) => {
  await emit(page, "show-overlay", {
    state: "recording",
    context: context("icons", {
      name: "Codex",
      icon: "codex",
      input_mode: "compose",
    }),
  });
  await expect(page.locator(".sprofile")).toBeVisible();
  const source =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
  await emit(
    page,
    "session-display",
    context("icons", { name: "Custom", icon: source, input_mode: "edit" }),
  );
  await expect(page.locator(".sprofile img")).toHaveAttribute("src", source);
  await expect
    .poll(() =>
      page
        .locator(".sprofile img")
        .evaluate(
          (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
        ),
    )
    .toBe(true);
  await expect(page.locator(".sprofile-edit")).toBeVisible();
  await expect(page.locator(".sdot")).toHaveCount(0);
});

for (const state of ["recording", "streaming"]) {
  for (const theme of ["light", "dark"]) {
    for (const placement of ["top", "bottom"]) {
      test(`${state} ${theme} ${placement} fits native logical bounds`, async ({
        page,
      }) => {
        await page.setViewportSize({
          width: state === "recording" ? 256 : 400,
          height: state === "recording" ? 50 : 120,
        });
        await page.evaluate(
          ({ theme, placement }) => {
            document.documentElement.dataset.theme = theme;
            (window as any).__position = placement;
          },
          { theme, placement },
        );
        await emit(page, "show-overlay", {
          state,
          context: context("1", {
            ...terminal,
            name: "Long profile name ".repeat(5),
          }),
        });
        await expect(page.locator(".sprofile")).toBeVisible();
        await expect(
          page.getByRole("button", { name: "cancel" }),
        ).toBeVisible();
        if (state === "streaming") {
          await emit(page, "stream-text-event", {
            committed: "Some words",
            tentative: "more words",
          });
          await expect(page.locator(".scard.open")).toBeVisible();
        }
        await expect(page.locator(".ov-stage")).toHaveClass(
          new RegExp(placement),
        );
        const icon = await page.locator(".sprofile").boundingBox();
        const wave = await page.locator(".swave").boundingBox();
        const cancel = await page.locator(".sx").boundingBox();
        expect(icon!.x + icon!.width).toBeLessThan(wave!.x);
        expect(wave!.x + wave!.width).toBeLessThan(cancel!.x);
        expect(icon!.y).toBeGreaterThanOrEqual(0);
        expect(icon!.y + icon!.height).toBeLessThanOrEqual(
          page.viewportSize()!.height,
        );
        if (state === "streaming")
          await emit(page, "stream-phase-event", {
            phase: "working",
            kind: "polishing",
          });
        else
          await emit(page, "show-overlay", {
            state: "processing",
            context: context("1", terminal),
          });
        await expect(page.locator(".sspinner")).toBeVisible();
        await expect(page.locator(".sprofile")).toHaveCount(0);
      });
    }
  }
}
