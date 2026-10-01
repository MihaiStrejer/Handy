# Context profile validation

## Baseline (2026-09-27)

Baseline commit: `8f9cf53cd1410cda26beea39ff802ac306e39585`, branch `feat/context-profiles`. The running installed application is `C:\Users\strej\AppData\Local\Handy\handy.exe`; it is not a build of this checkout. No ordinary dictation, post-processed dictation, or microphone cancellation has been observed in this implementation session. Those native checks remain unverified.

### Existing pipeline

- `src-tauri/src/actions.rs::TranscribeAction::start` starts model loading, chooses the streaming mode, shows the overlay, then starts recording. Microphone readiness arrives separately after real input samples.
- `TranscribeAction::stop` retains the audio manager's cancellation generation, stops recording, transcribes, then calls `process_transcription_output`. Ordinary mode applies language conversion; post-processing additionally calls the selected provider. `build_system_prompt` strips `${output}` because the transcript is a separate user message. Legacy post-processing failures can return the original transcript.
- `complete_unless_cancelled` polls cancellation every 25 ms. The output path also checks cancellation before history and inside the main-thread paste closure. `utils::cancel_current_operation` increments the audio generation, cancels streaming, hides the overlay, and notifies the coordinator.
- History is written before output dispatch. `clipboard::paste` returns `Result<(), String>` after dispatch and can return success even for `PasteMethod::None`. The reliable-paste path can return before asynchronous delivery completes. None of these results proves insertion into the intended field.
- `commands/history.rs::retry_history_entry_transcription` transcribes a stored recording and updates history. It does not paste or have a live target. Profile capture must stay outside this entry point.

### Candidate Windows controls

All rows are candidates, not supported or verified integrations. Capture tests must use synthetic fixture text, including non-ASCII characters and multiline selections.

| Target                           | Required check                                                             | Current evidence |
| -------------------------------- | -------------------------------------------------------------------------- | ---------------- |
| Native Win32 edit control        | Caret, selection, surrounding text, focus change during lookup             | Not tested       |
| Edge textarea/contenteditable    | Multiline and non-ASCII selection, UTF-16 boundaries, empty caret          | Not tested       |
| Edge password input              | Protected state with no returned field content                             | Not tested       |
| VS Code editor                   | Accessibility enabled/disabled; explicit unsupported result when needed    | Not tested       |
| Windows Terminal with two tabs   | Focused tab's workspace, distinguish other tab/process working directories | Not tested       |
| VS Code integrated terminal      | Workspace and focused terminal association                                 | Not tested       |
| Unsupported/custom-drawn control | Unavailable rather than an invented empty selection                        | Not tested       |

### Widget baseline

`src/overlay/RecordingOverlay.tsx::listeningRow` shares `.sbase-l` between compact and Live recording: dot left, waveform center, timer/Cancel right. The dot is neutral while arming and accented/pulsing only after `recording-ready`. Working rows put a spinner in the same left slot. `src-tauri/src/overlay.rs` defines logical native windows of 256 by 50 for compact and 400 by 120 for streaming. CSS defines a 40-pixel base row, 172-pixel resting pill, 184-pixel Live pill, 216-pixel working pill, and 392-pixel expanded panel. Profile identity must replace the dot without changing microphone readiness or the working spinner.

### Tooling and checks

- Bun 1.3.11, Cargo/Rust, CMake, Clang, Windows SDK, and `uv` are available. The Rust host is `aarch64-pc-windows-msvc`; this repository uses its CPU backend on Windows ARM64, so Vulkan is not required.
- `bun install --frozen-lockfile`: passed, 341 packages; lockfile unchanged.
- `bun run build`: passed; existing Vite large-chunk warning.
- `bun run lint`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml transcription_coordinator --locked`: failed before compiling Handy: native dependency rejects MSVC for ARM. Existing Clang/Ninja with the repository CI settings resolves this. A second attempt required the already installed VC runtime staging path.
- No native desktop-control capability is exposed in this session. Browser automation can verify settings and mocked events, but cannot establish native microphone, accessibility, paste, or Windows scaling behavior.

## T01 verification

`cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 276 passed, including 6 context-session tests. The new tests reject stale resolver results after cancel or a newer session, repeated sends/results, changed snapshots, and model-supplied completion/profile authority. The native target adapter compiles for Windows ARM64, but actual focused-control capture remains unverified. No settings or endpoint behavior is enabled by T01.

Commands use the same environment as `.github/workflows/build.yml` for Windows ARM64, with locally installed tools and runtime:

```powershell
$env:CMAKE_GENERATOR='Ninja'
$env:CC='clang-cl'
$env:CXX='clang-cl'
$env:CMAKE_C_COMPILER_TARGET='aarch64-pc-windows-msvc'
$env:CMAKE_CXX_COMPILER_TARGET='aarch64-pc-windows-msvc'
$env:CL='/EHsc'
$env:RC='llvm-rc'
$env:TRANSCRIBE_CMAKE_ARGS='-DGGML_NATIVE=OFF -DGGML_OPENMP=OFF'
$env:HANDY_VC_REDIST_DIRS='C:/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/VC/Redist/MSVC/14.50.35710/arm64/Microsoft.VC145.CRT'
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
```

The first parser test run found that Serde accepted unknown fields in an internally tagged unit variant. Changing the no-effect variant to an empty struct variant makes those fields fail validation. The rerun passed. Session lifecycle APIs have dead-code warnings until the capture/routing/request tasks wire their consumers; existing unrelated warnings remain unchanged.

## T02 verification

`cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 282 passed. `cargo build --manifest-path src-tauri/Cargo.toml --bin handy --locked` and `bun run build`: passed. Running `handy.exe --list-devices` from `src-tauri` regenerated `src/bindings.ts` and reported the Snapdragon X Elite CPU backend.

A development build with `TAURI_CONFIG={"identifier":"com.pais.handy.context-dev"}`, a `Handy Portable Mode` marker in `src-tauri/target/debug/portable`, and `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223` used only `src-tauri/target/debug/Data`. The installed Handy process and its settings were unchanged. Native WebView checks passed create, rename, override, reset, General deletion rejection, stale edit rejection, dictionary collision rejection, flag dependency, atomic flag clear, and dictionary retention. After restarting the development process, the renamed profile, inherited prompt state, dictionary, and disabled flags remained; deletion then passed. Memory stayed empty and process-local. `node scripts/verify-context-profile-storage.mjs` and, after restart, `node scripts/verify-context-profile-restart.mjs` reproduce these checks while Vite serves port 1420. Both scripts reject any other data directory. Onboarding is marked completed only in this isolated fixture to prevent automatic model downloads.

The binding-export test experiment made the test executable retain Windows GUI imports without the application's Common Controls manifest, causing `STATUS_ENTRYPOINT_NOT_FOUND` for `comctl32!TaskDialogIndirect`. Removing that experiment and exporting through the normal application resolved it. The apparent zero-byte DirectML file was a symbolic link, not a missing DLL. The dependency cache was restored from the original SHA-256-verified ort-sys archive after investigating it; no replacement runtime or test exporter remains in source.

Profile configuration is stored separately in `context-profiles.json`. Revisions apply to the whole catalog; stale saves require a reload. First-use initialization copies the selected effective prompt once, removes `${output}`, and now uses a bounded built-in template when the legacy selection is absent or invalid, so the initially hidden Profiles section cannot become impossible to enable. Empty or malformed saved prompts cannot enable profile mode. Initial storage bounds are 64 profiles, 16 rules per profile, 500 keywords per profile, 50 misheard forms per keyword, 120 characters per keyword/form, and 32000 characters per prompt. T06 still needs aggregate request bounds and endpoint compatibility checks. Until then, an explicitly enabled profile request returns a recoverable error without pasting raw speech over a selection.

## T03 verification

`cargo test --manifest-path src-tauri/Cargo.toml --lib context_profiles --locked`: 16 passed. Two tests create real native Windows Edit controls with their own message pump, offscreen and without activating the user's window. They verify a selected `Codex` plus emoji, an empty selection and UTF-16 caret offset 15, a multiline selection preserving CRLF, protected-field exclusion, and rejection when the captured handle is not the foreground input. These establish the native Edit adapter on this machine; browser editors, VS Code, terminal selections, and complete microphone sessions remain unverified.

This native provider did not expose `TextPattern` in the fixture. The adapter checks native password style and UIA identity/protection, then uses TextPattern if available or bounded `EM_GETSEL`/`WM_GETTEXT` messages for an exact native Edit HWND. It reads selection and text twice to reject changes during capture. Shared render HWNDs are excluded because they do not identify one input. Application identity is an executable basename; private window/process/thread handles remain outside serialized context.

The capture ticket has a 250 ms deadline and admits at most one outstanding provider thread. A hung provider causes later lookups to return unavailable until it exits, rather than accumulating threads. Late results cannot attach after cancellation or a new session. Selection is bounded to 2048 Unicode scalar values; surrounding text is a document prefix bounded to 4096. Native messages each have a 25 ms timeout. Selection outside the bounded native buffer or truncated selection is uncertain, never an empty selection. Caret offsets are UTF-16 units within the captured prefix and unavailable if outside it. These are conservative initial limits; endpoint request-budget validation remains T06.

## T04 routing and deferred workspace integration

Nineteen context-profile tests pass, including workspace/app/General precedence with synthetic workspace evidence, exact Windows path comparison, ambiguous rules, and prompt snapshots surviving edits/deletion. Live production capture never supplies a workspace yet. The user selected Windows Terminal and approved application routing first on 2026-09-27.

Microsoft's [Terminal automation peer source](https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalControl/TermControlAutomationPeer.cpp) exposes a name/help text based on terminal titles and delegates text ranges; the inspected methods do not expose a current-directory identity. Microsoft's [same-directory shell integration guide](https://learn.microsoft.com/en-us/windows/terminal/tutorials/new-tab-same-directory) describes the shell reporting its directory to Terminal, not an external Handy query bound to the focused tab. No verified focused-tab workspace interface was established from these sources. Workspace support is deferred by the user's decision, rather than inferred from a title or process tree.

## T05 widget evidence

Nine browser tests cover profile replacement, microphone readiness, stale events, unknown input mode, both layouts, light/dark themes, top/bottom placement, long names, generic icons, spinner transitions, and logical native bounds. `PLAYWRIGHT_CHANNEL=msedge` uses installed Edge because Playwright Chromium is absent. `node scripts/verify-context-profile-overlay.mjs` exercised native events in the actual 256 x 50 overlay at device scale 1 and saved `design/proof/profile-overlay-native.png`. This is an event fixture, not a complete voice session. Windows 150% display scaling and human live acceptance are outstanding.

## T06 endpoint evidence

The existing Rust chat-completions client now carries the resolved profile request. Loopback HTTP tests inspect the actual model, system/user roles and response schema. Parser tests reject raw text, fenced JSON, unknown fields, empty/oversized output, invalid effects and selection-operation mismatches. Context remains in one bounded user-data envelope. Selected-text output rereads the supported field before dispatch and refuses changed selection, surrounding text or caret; Windows dispatch also compares the original native focus identity. This checks the destination before dispatch and does not prove completed insertion for learning.

`node scripts/verify-context-profile-endpoint.mjs` passed a synthetic compatibility probe against OpenAI `gpt-6-luna` through the isolated native Tauri command in 2484 ms. The test used the existing experiment key, sent no private capture, and restored the isolated endpoint settings. Its timing is for the small probe only. Model-proposed learning remains unapplied until T07's completion policy is settled. History retries cannot acquire a current profile session.

## T08 settings evidence

Four browser tests cover inheritance, cancelled customization, invalid-draft retention, variable insertion/focus, custom save/reset, General changes, dictionary layout/collision retention, profile isolation, memory removal/clear, keyboard tabs, and minimum-width layout. Native UI automation at 680 x 570 and 100% scale created two profiles, saved a dictionary and custom prompt, disabled the parent flag, and verified hidden-section navigation. After stopping and restarting only the development process, `node scripts/verify-context-profiles-ui.mjs --restart` confirmed persistence and removed the fixtures. Screenshots: `design/proof/profiles-native-dictionary.png` and `design/proof/profiles-native-context.png`.

A renderer resize request was rejected by Tauri's `core:window:allow-set-size` permission. The native window was already 680 x 570, so no permission change or elevation was needed. The test imports the already loaded Vite store URL when refreshing settings; importing a second hot-reload URL would create a separate store and fail to update the visible app.

## Verification and remaining gates

The complete Cargo suite passes 295 tests, including the preflight regression test. Frontend build, lint, 15 browser tests, and translation-key checks pass. New entries use English fallback text in other locale files; this is key coverage, not completed localization. Cargo clippy passes with existing warnings in audio permissions, transcription and paste transaction code. Repository-wide `bun run format:check` reports 212 files, including 186 unchanged tracked files and existing experiment/design artifacts; the feature's changed sources are formatted separately. No blanket formatter was applied to unrelated files.

T07 is blocked on the pending completion-policy choice. T09 still needs a real voice session, trustworthy learning evidence under that policy, supported-control review, and the user's widget/selection acceptance. Application routing is implemented; verified Windows Terminal workspace detection remains explicitly deferred.

Final lifecycle review also added the unique session-ID check at queued paste dispatch. A successful newer recording can share a cancellation generation with the previous one; generation alone must not authorize an older callback. The callback ignores that stale result without hiding the newer overlay.
