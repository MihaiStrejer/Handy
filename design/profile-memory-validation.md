# Profile memory validation

Evidence separates local validator/fixture behavior, browser mocks, native controls and real endpoint quality. Task status is authoritative in [plan.md](../plan.md#profile-memory-implementation-task-graph).

## Dictionary cleanup audit

- Active backend memory types, frontend bindings/editor, proposal contract and current spec use long-term text. No runtime legacy placeholder alias exists.
- `context_profiles/migration.rs` is the isolated schema-1 decoder and literal-placeholder conversion boundary. Its legacy names and tests are intentional. Exact migration backups retain the original names and data.
- History translation keys `add_misheard_form` and `remember` label previously saved rows as legacy proposals. New runs use `memory_changes`. Historical database values are not rewritten.
- Obsolete profile translation keys and keyword-row CSS were removed. ASR `custom_words` remains independent.
- Six old native schema-1 scripts and the architecture generator now print retirement notices. The icon verifier uses the narrow current DTO. The static architecture-page verifier explicitly tests historical evidence, not the current API.
- Architecture pages have a visible historical banner. `design/alternatives` and `design/mockups` are historical design evidence, classified by their README; their static source/examples remain with the screenshots. Earlier T00-T09 and checkpoint entries in plan.md retain historical evidence/status. MR acceptance/audit text names the removed API to define cleanup.
- Pending processing-message documents now consume free-text memory and `memory_changes`, retaining their distinct prompt/capture/output changes. PM01 migrates actual schema 3 to reserved schema 4; no invented schema-2 migration is shipped.

## Current verification

CPU-only Windows verification uses `TRANSCRIBE_CMAKE_ARGS=-DTRANSCRIBE_VULKAN=OFF -DGGML_CPU_ALL_VARIANTS=OFF`; production build configuration is unchanged. Native DLLs must be staged beside test executables. The final full Rust suite passed 343 tests, with the optional evaluator ignored. Browser checks cover profile isolation, instructions, promotion/navigation, cancellation, Undo, empty/missing-model states, retry, draft races, unmount and compact long-label layout.

Real microphone dictation and real-provider quality are not established by these fixtures. The installed Handy application and its data were not modified.

## Reproducible evaluation

Run `node scripts/evaluate-profile-memory.mjs fixture` with the native build prerequisites available. On this Windows machine, set the Python CMake tools on PATH and `TRANSCRIBE_CMAKE_ARGS=-DTRANSCRIBE_VULKAN=OFF -DGGML_CPU_ALL_VARIANTS=OFF` before Cargo. The runner stages existing test DLLs only and runs an ignored Rust evaluator against real local HTTP responses and the production client/parser/validator. It writes [evaluation-fixture.json](proof/profile-memory/evaluation-fixture.json).

Corpus v1 has 25 rewrite cases, each through JSON and native tool transport (50 requests), plus three independent consolidation requests. The completed fixture run matched every annotation: zero unsupported locally eligible cases, all annotated positive cases eligible, and one request per warm rewrite. Rewrite p50 was 0.781 ms and p95 1.122 ms on this run; these are localhost fixture timings, not model latency. The initial run exposed one changed-mind case accepted in both protocols; the validator now excludes explicit temporary-message/draft and changed-mind cues, and the complete corpus passes.

Two promotion examples preserve old terminology/scoped context. The third intentionally demonstrates a valid JSON response losing a prior fact: the evaluator reports a retention failure even though production parsing accepts its structure. Literal fragment checks detect the known loss; they cannot prove semantic retention or validate arbitrary generated prose.

For optional real-model evaluation, set `PROFILE_MEMORY_EVAL_URL`, `PROFILE_MEMORY_EVAL_MODEL` and, if needed, `PROFILE_MEMORY_EVAL_KEY`, then run `node scripts/evaluate-profile-memory.mjs endpoint`. It sends only this synthetic corpus, does not load or modify user profiles/settings, and writes a separate endpoint report. Annotations and timing are recorded by actual protocol; fixture success does not predict model recall. The selected installed-app endpoint had an empty model and no credential when checked, so no real-model run was available. Conservative language patterns are not a multilingual semantic classifier, and unsupported languages/statements can have low recall.

## Native proof and recovery

`node scripts/verify-profile-memory-native.mjs backend` passed 55 profile tests, with the optional evaluator ignored. It exercises real offscreen Windows Edit selection/readback and session-scoped batch admission. The integrated correction test then sends the next rewrite to a real local HTTP fixture, runs independent consolidation through a second HTTP fixture, reloads the persisted catalog, and verifies Undo after another reload. Both profiles remain isolated and short-term notes remain available. The controls are owned by the test, and no user clipboard or microphone is touched.

`ui` and `restart` phases connect to an isolated Tauri development WebView at CDP port 9223. They require `src-tauri/target/debug/Data/.profile-memory-verification-owned`, the `portable` marker and the checked schema-1 migration fixture. Build with the temporary environment override `TAURI_CONFIG={"identifier":"com.pais.handy.profile-memory-verification"}` so verification cannot forward to the installed application's single-instance channel. This is a test-build override, not a committed application identifier change. Launch with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223`, `--start-hidden --no-tray`, and Vite on port 1420. Set onboarding complete and profile flags only in the disposable settings file. Stop this owned app before Cargo builds because Windows locks its runtime DLLs.

Actual native UI/command checks passed: direct migration to schema 3, exact original backup hash, literal memory preservation, two-profile isolation, narrow-save rejection of injected memory, instruction save/restart and update-button placement to the right. Empty short-term memory rejects explicitly before HTTP. The screenshot and command reports are [native-ui.png](proof/profile-memory/native-ui.png), [native-ui.json](proof/profile-memory/native-ui.json), [native-restart.json](proof/profile-memory/native-restart.json) and [native-backend.json](proof/profile-memory/native-backend.json).

Recovery review found that the store plugin uses a direct overwrite. Profile persistence now has a dedicated file owner: serialize the existing root with the new catalog, write and sync an owned same-directory temporary file, then replace the destination before publishing the cache. Profile files are not registered with plugin autosave/exit writes. Existing top-level values are retained. A real partial-write failure test confirms exact original bytes survive, owned temporary files disappear, and a failed first write creates no destination. Duplicate profile IDs and a reused next ID reject before migration/publication. The existing tempfile dependency is used in production for this atomic replacement; ordinary settings still use the store plugin.

Normal microphone dictation and a live model-driven correction followed by button consolidation have not been observed. The local fixtures and native command proof do not close those desktop/model gates or prior T09/PM review requirements. Unsupported controls/platforms continue to withhold correction learning without verified readback.

## Final checks and cleanup

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --quiet`: 343 passed, 0 failed, 1 ignored.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --quiet`: passed with warnings, including existing warnings and argument-count warnings in the completion API.
- `cargo build --manifest-path src-tauri/Cargo.toml`: passed with the default application identifier restored. This is a CPU-only Windows debug executable; a production installer was not built.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed.
- `bun run build`, `bun run lint`, `bun run check:translations`: passed; all 25 translated locales have the current keys. New fallback text is English, not a claim of translation quality.
- `PLAYWRIGHT_CHANNEL=chrome bun run test:playwright`: 36 passed, including profiles, overlay and History browser fixtures. Browser fixtures do not establish physical microphone or installed-app behavior.
- Repository-wide `bun run format:check` still reports 313 files in this Windows checkout, primarily existing CRLF formatting. Changed current application, scripts, JSON and Markdown files pass a separate Prettier check. The two historical architecture HTML files retain their existing formatting; their only change is a supersession banner. Generated bindings are validated by the build and trailing-whitespace check.

The owned native app was stopped before rebuilding, and its Vite process was stopped after the proof. Automatic approval review rejected removal of the disposable portable `Data` directory/marker with “blocked by policy”; those marked fixtures remain under `src-tauri/target/debug`, so cleanup is incomplete. Do not launch that executable with the remaining portable marker unless using the disposable fixture deliberately. The installed application's settings, profiles, history and migration backups were untouched. MR09 retains a blocked desktop/model verification and cleanup gate; earlier T09 and pending PM task statuses are preserved.

## Context-provider follow-up (2026-10-04)

The memory implementation is included in the same feature-branch delivery as the new application extractors. [Provider scope and verification](implementation/context-providers.md) records directory-first routing, default/General fallback, editable templates and started-executable T3/Claude proof. Final combined regression count is 351 Rust tests passed (one optional evaluator ignored) and 38 Chrome Playwright tests passed. Frontend build, ESLint, 714 translation keys across 25 translated locales, Clippy, Rust formatting, changed-file formatting and whitespace checks pass. Global Prettier reports 305 baseline paths; none is modified by this delivery. Native debug build uses the documented CPU-only CMake override; a Vulkan-enabled production installer is not verified here.

The new proof is isolated outside the repository at `D:/Tools/Handy-qa/2026-10-04_feat-context-profiles_started-app/index.md`, with three human verdicts pending. It does not close MR09's physical microphone/real-model or previous fixture-cleanup gates. The old marked portable Data fixture remains because automatic approval review rejected deletion with "blocked by policy"; this delivery does not retry or bypass that rejection. The installed Handy process and user store are unchanged. New disposable observers exit, and the owned default-provider Forms window was closed.
