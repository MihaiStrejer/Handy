# Context profiles: implementation plan

Status: Implementation started; canonical task graph for the [intent](intent.md) and [specification](spec.md). The first delivery adds context capture, profile routing, inherited or custom system prompt templates, the recording-widget indicator, dictionary and memory management, and a structured request handed to Handy's configured post-processing endpoint. It does not add a local text model, a second provider path, or a new insertion coordinator.

Supersession: The [processing-message graph](#processing-message-implementation-task-graph) and its specification replace D7's inheritance/placeholder rules for the next delivery. Earlier T-task decisions and completed evidence remain historical records; PM01–PM06 are separate pending work.

## Current code boundaries

- [`actions.rs`](src-tauri/src/actions.rs) starts and stops dictation, processes the transcript, writes history, and calls `utils::paste`. Its post-processing helper currently returns `Option<String>` and can fall back to the transcript on failure. Profile mode needs a distinct result so an invalid structured action cannot become pasted text.
- [`llm_client.rs`](src-tauri/src/llm_client.rs) already sends chat-completion requests. The current structured schema contains only `transcription`; support varies by provider. Profile mode must check that the selected endpoint accepts the context request and returns the required text and action when processing a request, without gating local profile setup. Apple Intelligence is outside this endpoint-only path unless it gains the same contract.
- [`settings.rs`](src-tauri/src/settings.rs) persists `AppSettings`, including `post_process_enabled` and the existing `custom_words` ASR hints. Post-processing settings commands are in [`shortcut/mod.rs`](src-tauri/src/shortcut/mod.rs), and [`lib.rs`](src-tauri/src/lib.rs) exports Tauri bindings. Context-profile dictionaries are separate from `custom_words`.
- [`overlay.rs`](src-tauri/src/overlay.rs) emits recording state to [`RecordingOverlay.tsx`](src/overlay/RecordingOverlay.tsx). The overlay has no session-scoped profile or edit-state event today. [`Sidebar.tsx`](src/components/Sidebar.tsx) shows Post Process when `post_process_enabled` is true.
- [`clipboard.rs`](src-tauri/src/clipboard.rs) reports whether an output action was dispatched. That result does not prove which field received the text. The existing history retry in [`commands/history.rs`](src-tauri/src/commands/history.rs) has no live target and must not acquire one while reprocessing old audio.

## Delivery decisions

Record these choices in the specification before implementing the dependent task. The defaults below allow work to proceed while preserving explicit unsupported states.

| ID  | Decision and proposed first-release rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Needed by               |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| D1  | Use the existing selected HTTP post-processing endpoint only when a tested model accepts the two-part response. Reject incompatible providers in profile mode; do not use the current raw-text fallback for a malformed action. Keep ordinary post-processing unchanged when profiles are off.                                                                                                                                                                                                                              | T01, T06, T09           |
| D2  | Start with Windows UI Automation for focused-input capture. Return explicit `unavailable` on platforms and controls that have not passed a real capture matrix; add Linux AT-SPI and macOS readers only after platform tests. Never use the clipboard as a context reader.                                                                                                                                                                                                                                                  | T01, T03, T09           |
| D3  | Route by verified workspace, then application, then General. A terminal workspace rule runs only when the directory belongs to the focused terminal session or tab. Otherwise use the application or General rule and show that choice in the widget.                                                                                                                                                                                                                                                                       | T01, T04, T09           |
| D4  | Keep one locally owned dictionary per profile. The left-side key is a canonical keyword, such as `Codex`; its right-side values are observed misheard phrases, such as `codecks` and `code X`. A phrase cannot silently point to two keywords in one profile. Do not migrate `custom_words`.                                                                                                                                                                                                                                | T02, T07, T08           |
| D5  | Admit a proposed `add_misheard_form` only when the keyword exists in the session's profile, the phrase is bounded, the phrase is observed in that session's transcript or captured selection, the canonical keyword appears in the result, no collision exists, and the output operation has a trustworthy completion signal. A generic replacement is not evidence. If Handy cannot verify completion, do not update either store. Exact normalization and size limits need tests with real examples.                      | T01, T07, T09           |
| D6  | Bound selected text, surrounding text, dictionary entries, and short-term memory separately. Exclude protected fields and disclose when context goes to an external endpoint. Use measured limits from T03 and endpoint tests in T06 before finalizing numbers. Keep memory profile-local and in process until a retention policy is chosen.                                                                                                                                                                                | T03, T06, T07, T08      |
| D7  | General owns the default system prompt template for profile mode. A profile with no override inherits General by reference; a custom template affects only that profile. Start with the named placeholders in the specification, reject malformed or unknown names, and do not enable template helpers or code. Proposed first-run behavior copies the effective selected Post Process prompt into General once, stripping legacy `${output}` as `build_system_prompt` does today; the selected provider remains unchanged. | T01, T02, T04, T06, T08 |

## Execution rules

This file is the canonical task graph. Task IDs T00 through T09 are retained from the earlier plan. Task status and evidence below track implementation; planning and mockups are not implementation evidence. Paths and commands below are relative to the repository root, `D:\rust\Handy`. New module and test paths are proposed locations, to be confirmed against project conventions when implementation begins. Source documents are [intent.md](intent.md), [spec.md](spec.md), and the [prompt behavior record](design/components/profile-system-prompt.md).

The settled widget placement is the far-left dot slot: the identified profile icon replaces `.sdot` inside `.sbase-l`. The waveform and Cancel retain their positions. This applies to compact and Live recording. See the specification's Recording widget placement section.

Complete each task's acceptance checks and record evidence here before marking it done. D1 through D7 are proposed implementation defaults, not additional task IDs. Record technical choices and measured limits in the specification as they are resolved. T07 records the verified field-readback decision for output completion; T09 includes a real desktop review. The user authorized implementation on 2026-09-27. No upstream PR is authorized.

The dependency order is T00 -> T01 -> T02/T03 -> T04 -> T05/T06, with T06 enabling T07 and T08, and T09 joining T05, T07, and T08. The learning-completion policy was resolved on 2026-10-01. T02 and T03 have largely separate implementation files but share command registration and bindings; T05 and T06 share session integration in `actions.rs`. Start sequentially. Only overlap work after those shared edits have a single owner and the verification resources are isolated. Parallelizable tasks do not authorize delegation.

## T00: Establish the existing behavior and target matrix

- Status: done
- Depends on: none
- Mode: AFK
- Files: `plan.md`; proposed `design/context-profile-validation.md`; read `src-tauri/src/actions.rs`, `src-tauri/src/clipboard.rs`, `src-tauri/src/commands/history.rs`, `src-tauri/src/overlay.rs`, `src/overlay/RecordingOverlay.tsx`, `src/overlay/RecordingOverlay.css`.
- Source: `spec.md` sections: Scope and existing integration, Voice-session behavior; `plan.md` sections: Current code boundaries.
- Acceptance:
  - [x] Record the ordinary and post-processing paths, cancellation boundary, prompt handling, output-dispatch result, and history retry behavior with code references.
  - [x] Name representative Windows input controls and terminal hosts for selection, caret-only input, protected fields, unsupported capture, and focused terminal tabs. Separate verified support from candidates.
  - [x] Record the compact/Live widget's dot, waveform, Cancel, readiness, processing spinner, and native window dimensions.
  - [x] Record available build/test tooling and baseline failures without treating missing tools as permission to install them.
- Verification: On a running Handy instance, observe one ordinary dictation, one post-processed dictation, and cancellation; record which parts were observed and which remain unverified. Capture must be evaluated on the actual target applications, not inferred from a successful build.
- Evidence: [Baseline and candidate matrix](design/context-profile-validation.md). Frontend build and lint pass; native behavior is explicitly unverified and Rust baseline compilation is still running. T00 records verification availability; it does not establish capture support.

## T01: Keep one context session through recording and processing

- Status: done
- Depends on: T00
- Mode: AFK
- Files: `src-tauri/src/actions.rs`, `src-tauri/src/transcription_coordinator.rs`, `src-tauri/src/lib.rs`; proposed `src-tauri/src/context_profiles/` session types and tests; generated `src/bindings.ts` when exported types change.
- Source: `spec.md` sections: Voice-session behavior, Request and feedback contracts.
- Acceptance:
  - [x] Create the session at dictation invocation and tie its lifecycle to the existing cancellation generation. Capture the destination identity before Handy changes focus; slow enrichment must not delay microphone startup.
  - [x] Carry capture availability, profile/prompt revisions, dictionary and memory snapshots, and request/result state through the existing action path. Private target data stays outside the endpoint payload.
  - [x] Define text operation and feedback effect separately, using the illustrative schema only as a proposal. Keep the profiles-disabled path working.
  - [x] Test a fake resolver completing after cancellation or after a newer recording: neither completion may replace the active session or trigger a request/update.
- Verification: Run the new Rust session tests with `cargo test --manifest-path src-tauri/Cargo.toml context_profiles`; confirm the filter actually runs the new tests. Exercise the existing cancellation regression tests affected by the change.
- Evidence: Session/target modules plus start, stop, cancellation, and queued-paste lifetime integration. `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 276 passed, including 6 context session tests and existing coordinator/cancellation regressions, using the Windows ARM64 CI environment recorded in the validation document. Platform enrichment, profile persistence, and HTTP use of the snapshot remain T02-T06; native dictation is unverified.

## T02: Persist profiles and enforce the feature flag

- Status: done
- Depends on: T01
- Mode: AFK
- Files: `src-tauri/src/settings.rs`, `src-tauri/src/shortcut/mod.rs`, `src-tauri/src/lib.rs`, `src/stores/settingsStore.ts`, generated `src/bindings.ts`; proposed profile storage module under `src-tauri/src/context_profiles/`.
- Source: `spec.md` sections: Settings and navigation, Long-term dictionary mockup; D4, D7 in this plan.
- Acceptance:
  - [x] Default `post_process_profiles` to false. Reject enabling it when post-processing is off; disabling post-processing clears the profile flag atomically and preserves profile data.
  - [x] Persist stable profile identities, match rules, dictionary revisions, General's default template, and optional prompt overrides. Record icon identity/source for the recording indicator; a missing asset has a generic fallback.
  - [x] Store a canonical keyword with a list of observed misheard forms. Keep ASR `custom_words` independent. Reject duplicates or collisions without losing existing entries.
  - [x] Round-trip create, rename, edit, and delete operations through Tauri commands and persistence. Define General's protected default role so deletion cannot strand inherited prompts.
  - [x] Test old settings, repeated migration, prompt override/reset, conflicting updates, and preservation of existing settings. Keep short-term memory in process under D6.
- Verification: Run `cargo test --manifest-path src-tauri/Cargo.toml settings` and the new profile storage tests. Reload the settings store after each flag transition and verify both flags and retained profiles.
- Evidence: 282 Rust library tests pass (6 added since T01). Generated bindings through the compiled application; frontend build passes. Native Tauri command checks in an isolated portable development instance passed create/rename/delete, General protection, override/reset, stale revision and dictionary-collision rejection, both flag transitions, retained data, and restart persistence. See `scripts/verify-context-profile-storage.mjs`, `scripts/verify-context-profile-restart.mjs`, and the validation record. Profile requests currently fail closed until T06; no profile UI is exposed yet.

## T03: Capture focused Windows input with explicit availability

- Status: done
- Depends on: T01
- Mode: AFK
- Files: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/actions.rs`, `src-tauri/src/lib.rs`; proposed `src-tauri/src/context_profiles/capture/` platform reader and tests.
- Source: `spec.md` sections: Voice-session behavior, Data handling and failure states; D2, D6 in this plan.
- Acceptance:
  - [x] Read the focused application and available selection, caret, and surrounding text from Windows accessibility, with capture timestamp, bounds, truncation, and per-field availability.
  - [x] Exclude protected field contents and distinguish an empty selection from capture failure. Never substitute clipboard text.
  - [x] Bound lookup time and text size. Record the actual chosen limits and validate that late capture cannot inspect a newly focused unrelated input.
  - [x] Provide explicit unsupported results for unverified controls/platforms. Preserve ordinary dictation when profiles are off.
  - [x] Establish evidence on T00's controls, including non-ASCII text and a multi-line selection; record the coordinate/offset semantics the adapter uses.
- Verification: Run fake-reader tests for empty, partial, protected, timed-out, truncated, and stale results. On Windows, compare captured selection and surrounding text with known fixture content in each supported control; no protected content may appear in the result or logs.
- Evidence: 16 context-profile tests pass, including real Windows native Edit fixtures for selection, a UTF-16 caret after an emoji, multiline selection, password exclusion, and stale focus; fake-reader deadline/single-worker tests and cancellation tests pass. Native Edit uses UIA identity/protection/TextPattern where available, with bounded documented Edit messages where TextPattern is absent. Other controls/platforms explicitly return unavailable; terminal workspace is not established. See the validation record for exact bounds and coverage.

## T04: Resolve the profile and its inherited prompt

- Status: done
- Depends on: T02, T03
- Mode: AFK
- Files: `src-tauri/src/actions.rs`; proposed matching and terminal resolver modules under `src-tauri/src/context_profiles/`; `spec.md` support matrix updates.
- Source: `intent.md` sections: Application-specific context; `spec.md` sections: Context selection mockup, Voice-session behavior.
- Acceptance:
  - [x] Match verified workspace, then application, then General with deterministic handling of equally specific rules. Workspace unavailability falls through implicitly.
  - [x] User scope update (2026-09-27): Windows Terminal is first; deliver application routing now and defer verified workspace integration. Production workspace capture stays unavailable. A window title or adjacent process is never used as evidence.
  - [x] Freeze the resolved profile, icon reference, match basis, effective prompt source/revision, dictionary, and memory snapshots for the session. General edits affect the next inherited request; overrides remain independent.
  - [x] Test two terminal tabs, two profiles for one application, ambiguous rules, deleted/edited profiles, missing icon assets, and focus changes during recording.
  - [x] Do not claim terminal workspace support. Record its deferred status and the inspected Windows Terminal interfaces; hypothetical workspace matcher tests do not establish a live integration.
- Verification: Run deterministic matcher tests and reproduce the focused-tab workspace case in T00's terminal matrix. Inspect the resolved snapshot and confirm it uses the same profile and prompt revisions throughout the session.
- Evidence: 19 context-profile tests pass, including precedence, exact-path matching, equally specific rules, and inherited/custom prompt snapshots after edits/deletion. Recording takes an immutable catalog reference and memory snapshot before asynchronous capture. User explicitly accepted application routing first and selected Windows Terminal; focused-tab workspace verification is deferred. Built-in icon identities avoid external asset paths; display fallback is tested in T05.

## T05: Replace the recording dot with the resolved profile icon

- Status: blocked
- Depends on: T04
- Mode: AFK
- Files: `src-tauri/src/overlay.rs`, `src-tauri/src/actions.rs`, `src-tauri/src/lib.rs`, `src/overlay/RecordingOverlay.tsx`, `src/overlay/RecordingOverlay.css`, `src/components/icons/`, `src/i18n/locales/en/translation.json`, generated `src/bindings.ts`; proposed `tests/context-profile-overlay.spec.ts`.
- Source: `spec.md` sections: Recording widget placement; `design/mockups/recording-profile.html`.
- Acceptance:
  - [x] Emit session-scoped profile/input-mode state and consume it in the existing overlay. Replace `.sdot` in `.sbase-l` when the profile is resolved in both compact and Live recording.
  - [x] Keep the dot for profiles-disabled or pending lookup states. Use the General or generic profile icon for a resolved fallback, with the exact profile name available as text to accessibility and hover.
  - [x] Keep microphone arming/ready feedback independent of profile resolution. Show verified edit state without covering the profile icon; unavailable selection must not be presented as verified compose.
  - [x] Keep waveform alignment and Cancel usable. Preserve transcribing/processing spinner behavior and the session's profile through the working phase.
  - [x] Clear stale profile state on hide, cancellation, and a new recording. Ignore late events from prior sessions.
  - [ ] Verify dark/light themes, compact/Live layouts, top/bottom placement, Windows scaling, missing icons, and long names against the native window bounds.
- Verification: Run `bun run test:playwright -- tests/context-profile-overlay.spec.ts` once the new test exists, using fake Tauri events for session ordering and readiness. Compare the running native overlay with the user's far-left-dot placement at 100% and 150% Windows scaling; browser screenshots alone do not establish native fit.
- Evidence: frontend build and lint pass. All 9 overlay browser tests pass using installed Edge (`PLAYWRIGHT_CHANNEL=msedge`); bundled Playwright Chromium is absent. Session ordering, readiness, generic icon, edit badge, hidden/off state, both layouts/themes/placements and logical window fit are covered. Native event-fixture check passed at 100% in the actual 256 x 50 overlay (`scripts/verify-context-profile-overlay.mjs`, screenshot in `design/proof/profile-overlay-native.png`). Windows 150% scaling and the live user review remain unverified; no system display setting was changed.

## T06: Render profile templates and call the existing endpoint

- Status: done
- Depends on: T04
- Mode: AFK
- Files: `src-tauri/src/actions.rs`, `src-tauri/src/llm_client.rs`, `src-tauri/src/commands/history.rs`; proposed template, request, and response modules/tests under `src-tauri/src/context_profiles/`.
- Source: `spec.md` sections: Context selection mockup, Request and feedback contracts; `design/components/profile-system-prompt.md`.
- Acceptance:
  - [x] Resolve the documented named placeholders once from the frozen session. Record exact serialization and unavailable markers; reject malformed/unknown expressions and do not evaluate helpers or context as template code.
  - [x] Keep the complete bounded request data available even if a custom template omits a placeholder. Specify message roles and avoid duplicating captured text through both interpolation and automatic sections.
  - [x] Use the configured HTTP provider/model through the existing post-processing path. Profile prompt selection must not silently change the endpoint.
  - [x] Request and strictly validate predicted text plus action. Schema mismatch, malformed JSON, empty results, and provider errors have explicit outcomes; raw response text must not bypass action validation.
  - [x] Verify provider compatibility and preserve ordinary post-processing when profiles are disabled. History retry never captures a new live target or invents an old profile snapshot.
  - [x] Test inherited/default/custom templates, legacy `${output}` initialization, bounded context, cancellation, no duplicate send, and sensitive-content redaction.
- Verification: Run Rust template/parser tests and mock HTTP endpoint tests that assert actual roles, payload sections, and failure behavior. Record one compatible endpoint/model result without putting credentials or raw private context in evidence.
- Evidence: 23 context-profile Rust tests pass, including real loopback HTTP role/schema/model assertions and malformed-response tests. `node scripts/verify-context-profile-endpoint.mjs` passed against OpenAI `gpt-6-luna` via the isolated native app: 2484 ms for the synthetic compatibility request. No private captured data was sent; prior endpoint configuration was restored. Request errors are redacted, profile requests fail closed, and history retry has no live target path.

## T07: Apply verified corrections to the originating profile's short-term memory

Scope update (2026-10-01): The user authorized automatic correction feedback into short-term memory and chose native field readback as the completion boundary. Long-term dictionary entries remain manually managed. This supersedes the earlier automatic dictionary-append requirement. PM01-PM06 remain separate pending work.

- Status: done
- Depends on: T06
- Mode: AFK
- Files: `src-tauri/src/actions.rs`; `src-tauri/src/context_profiles/{feedback,session,storage,capture,routing,request}.rs`; `src/components/settings/context-profiles/ContextProfilesSettings.tsx`; `tests/context-profiles.spec.ts`; `spec.md`.
- Source: `spec.md` Request and feedback contracts; the user's verified-completion decision and bomb/BOM example.
- Acceptance:
  - [x] Completion decision: successful paste dispatch alone cannot admit memory. Read back the same supported field and verify the entire expected result, collapsed selection, and caret. Unsupported or incomplete capture adds no memory.
  - [x] Return corrected text and a separate `remember` proposal in one response. A new term, such as BOM (bill of materials), needs no existing dictionary entry. A changed mind or generic replacement can have effect `none`.
  - [x] Retain legacy `add_misheard_form` validation for existing, unambiguous keywords, but convert an accepted proposal into short-term memory without modifying the dictionary.
  - [x] Keep memory bounded, profile-local, and in process. Reject duplicate delivery, stale/deleted profiles, changed dictionary identity, invalid effects, cancellation, and unverified output. Removing or clearing memory invalidates in-flight feedback based on the old memory snapshot.
  - [x] Include accepted memory in the next request and refresh the visible memory tab. Demonstrate correction/readback/admission/next-request behavior using a real supported native control.
- Verification: Rust feedback, session, storage, request, and Windows native Edit tests; browser memory refresh/removal/profile-isolation test; frontend lint/build and native build. The native fixture uses an offscreen Edit control and a deterministic model response; live microphone/third-party endpoint behavior belongs to T09.
- Evidence (2026-10-01): `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features` passed 322 tests (38 context-profile tests). The native Edit fixture replaced selected `bomb` with `BOM`, read back the full field and UTF-16 caret, admitted one correction, confirmed it in the next request with an unchanged empty dictionary, and verified clearing removes it. Tests also cover stale/cancelled/duplicate sessions, deleted/edited profiles, unknown/colliding dictionary keys, Unicode/range mismatches, bounds, deduplication, and clear-vs-in-flight feedback. `PLAYWRIGHT_CHANNEL=msedge bun run test:playwright tests/context-profiles.spec.ts --reporter=line` passed 14 tests, including live memory event refresh and profile isolation. `bun run build`, `bun run lint`, native `cargo build --manifest-path src-tauri/Cargo.toml --bin handy --locked --no-default-features`, cargo fmt check, and changed-frontend Prettier check passed; existing compiler and bundle-size warnings remain. The first fixture run failed on a UI tab locator and a UIA provider error; the corrected run uses the production native Edit reader. No live microphone, paid endpoint, browser-field, or terminal-field learning is claimed; native readback remains limited to supported Windows Edit controls.

## T08: Deliver the three-tab profile settings workflow

- Status: done
- Depends on: T06
- Mode: AFK
- Files: `src/components/Sidebar.tsx`, `src/components/settings/index.ts`, `src/components/settings/post-processing/PostProcessingSettings.tsx`, proposed `src/components/settings/context-profiles/`, `src/stores/settingsStore.ts`, `src/hooks/useSettings.ts`, `src/i18n/locales/en/translation.json`, generated `src/bindings.ts`; proposed `tests/context-profiles.spec.ts`.
- Source: `spec.md` sections: Settings and navigation, Context selection mockup, Long-term dictionary mockup, Short-term memory mockup; `design/mockups/context-profiles.html`; `design/components/profile-system-prompt.md`.
- Acceptance:
  - [x] Show the Profiles section only when both flags are enabled. Place the toggle in Post Process; disabling post-processing clears it without deleting profiles. Leaving a hidden section returns navigation to a valid section.
  - [x] Provide the left profile selector and Add profile, with exactly Context selection, Long-term dictionary, and Short-term memory tabs scoped to the selected profile.
  - [x] Context selection edits routing and shows General default, inherited, custom, editing, and invalid-template states. Save, Cancel, variable insertion, and reset-to-General work with the backend and preserve focus.
  - [x] Dictionary rows show the canonical key on the left and misheard forms on the right. Add/remove operations report collisions and source errors without losing the draft.
  - [x] Short-term memory shows accepted items and supports Remove/Clear for the correct profile. Verify this screen with seeded items and boundary event fixtures; T09 covers real feedback from T07.
  - [x] Use translation keys, generated bindings, and existing settings state conventions. Verify 680 x 570 layout, keyboard access, long content, state after app restart, and both flag transitions.
- Verification: Run `bun run test:playwright -- tests/context-profiles.spec.ts` after adding the test, plus `bun run build`, `bun run lint`, and `bun run check:translations`. In the native app, edit two profiles, restart, and verify dictionary/prompt persistence and the chosen in-process memory lifetime.
- Evidence: four profile UI browser tests pass. Native 680 x 570, 100% WebView testing passed two-profile creation, dictionary save, custom prompt, endpoint toggle, parent flag clear, and leaving a hidden section. The owned development process was restarted; both profiles, dictionary, and prompt survived, memory was empty, and fixtures were removed (`scripts/verify-context-profiles-ui.mjs` and `--restart`). Build and lint pass; all 25 locale files contain the keys, with English fallback text for new entries pending translation. Screenshots are in `design/proof/`.

## T09: Verify the full voice workflow on supported targets

- Status: pending
- Depends on: T05, T07, T08
- Mode: HITL
- Files: `plan.md`, `spec.md`, proposed `design/context-profile-validation.md`, relevant user documentation; integration tests for `src-tauri/src/context_profiles/` and `tests/`.
- Source: `spec.md` sections: Acceptance criteria; all task evidence above.
- Acceptance:
  - [ ] Record a complete voice session through capture, routing, icon replacement, transcription, effective template, configured endpoint, existing output, and validated feedback.
  - [ ] Verify application/workspace match, inherited/custom prompt behavior, selected-text editing, caret composition, unavailable context, protected fields, provider rejection, focus changes, and cancellation against the declared support matrix.
  - [ ] Confirm the icon, request, and feedback share a session/profile identity. No captured field content or private target token may enter ordinary logs/history.
  - [ ] Demonstrate the accepted dictionary/memory change on the next request and isolation between two profiles. Do not equate successful request assembly with working automatic learning.
  - [ ] Run the relevant clean build, lint, translation, formatting, Rust, and browser checks; distinguish baseline failures from regressions.
  - [ ] Human gate: review the running widget at the marked far-left slot and a real selected-text scenario, recording accept/reject and any remaining limitations before declaring the feature complete.
- Verification: From the repo root, run `bun run build`, `bun run lint`, `bun run check:translations`, `bun run format:check`, `bun run test:playwright`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`. Record commands, test counts, platform/app/model versions, screenshots, and the live-review result. Unsupported platform coverage remains explicit.
- Evidence: full Cargo suite: 295 passed; browser suite: 15 passed. Frontend and native builds, lint, translation-key coverage, and clippy pass (existing clippy warnings remain). Changed-source Prettier and cargo fmt checks pass. Repository-wide format check reported 212 files, including 186 unchanged tracked files; unrelated formatting was retained. T09 remains pending because T05 native 150%/human review and T07 completion policy/learning are not complete.

## Completion boundary

The request path is complete when a recorded session reaches Handy's configured post-processing endpoint with the correct frozen profile, effective inherited or custom prompt, and bounded context, and the widget shows the same profile and edit/compose state. The feedback hook is complete when it validates an action and applies it only after a trustworthy output-completion signal; if that signal is unavailable for a target, the UI must not claim that its dictionary or memory learned from that result. Existing insertion behavior stays under Handy's current post-processing path, and extending destination verification is a separate design decision if automatic learning must work in more controls.

## Profile setup correction (2026-09-27)

- Status: done
- User decision: allow profiles and dictionaries to be configured before a model or API key is selected. Keep the existing parent post-processing flag dependency.
- Changes: remove enable-time network validation; retain compatibility checks in the request path; update toggle text and architecture guidance. The historical enable-time endpoint verification script is retired because toggling no longer proves endpoint compatibility.
- Verification: `node scripts/verify-profile-setup-without-model.mjs` passed against the rebuilt native development app: enabling, creating a profile and saving a dictionary succeeded with an empty model/key; enabling with a configured model and a rejecting endpoint also succeeded; zero endpoint requests occurred. Parent flag rejection remains intact. All 24 context-profile Rust tests, frontend/native builds, lint, translation-key checks and cargo formatting passed. Test profile and temporary settings were restored. Compatibility checks remain in `request::process` before the captured-context request.

## Profile layout refinement (2026-09-27)

- Status: done
- User request: delegate to GPT-6-Sol; handle long profile names, replace the icon dropdown with visible icons, and extend the profile divider through the full available height with many-profile support.
- Implementation: responsive 176-224px profile sidebar, two-line name truncation with full-name titles, matching recording-widget icons, native radio selection, fixed heading/Add action, independent list/detail scrolling, and a bounded Profiles layout in `App.tsx`.
- Verification: seven profile browser tests pass, including keyboard icon selection/persistence and 27-profile fixtures at 680 x 570 (light) and 1050 x 760 (dark). The actual native 1049 x 737 window has no horizontal overflow and its divider reaches the footer; no user profiles or preferences were edited. Frontend build and lint pass. Evidence: `scripts/verify-profile-layout-native.mjs`, `design/proof/profiles-layout-native.png`, and `design/proof/profiles-layout-{680,1050}.png`.

## App icons and imported profile images (2026-09-27)

- Status: done
- Scope: 15 bundled icon choices including the requested apps, compact picker, and PNG/ICO imports that work consistently in the editor, selector and recording widget.
- Storage: validate and normalize imported raster images to a bounded PNG, then save their bytes within the profile catalog in Handy's data directory. The original file path is not retained and is not required after saving. Preserve existing built-in icon values.
- Verification: 28 context-profile Rust tests pass, including image validation and legacy/custom icon round trips. Native build passes. `node scripts/verify-profile-icons-native.mjs` confirms import, persistence after deleting the original image, and recording-widget rendering. Browser checks cover picker cancellation/errors, keyboard selection, and built-in/custom rendering. Evidence: `design/proof/profile-icons-native.json` and `design/proof/profile-custom-icon-overlay.png`.

## Profile autosave and prompt modal (2026-09-27)

- Status: done
- User decisions: icon tiles show only centered icons, with names in tooltips/accessibility labels. Ordinary profile fields, routing, icons and dictionaries autosave. The system prompt has its own modal: Save and close commits; Cancel, Escape and backdrop dismissal discard its private draft. This supersedes relocating a manual Save profile button.
- Acceptance: debounce ordinary edits, serialize writes using the current catalog revision, preserve newer typing and invalid drafts, flush valid pending changes when switching profiles, and keep failed prompt saves in the modal.
- UI refinement: remove the repeated profile title/description and Saved/Saving/Unsaved indicators. Text inputs save after a 400 ms debounce; errors remain visible. Tabs start at the top of the detail pane.
- Verification: all 23 profile/overlay browser tests pass using installed Edge, including slow responses, stale rejection recovery, switch/unmount flush, and prompt cancellation. `node scripts/verify-profile-autosave-native.mjs` passes against the native development app, checking persisted name/icon/dictionary edits and transactional prompt saves. Frontend build, lint, translation-key coverage and changed-source formatting pass. Temporary test data was removed. Evidence: `design/proof/profile-autosave-native.json`, `design/proof/profile-autosave-native.png`, and `design/proof/profile-prompt-modal-native.png`.

## Proposed history inspection extension (2026-09-27)

- Design status: Option B side inspector selected; header status and five failure samples complete. No application changes are included in this design slice.
- Source: [History post-processing details](design/components/history-post-processing.md), with integration points, archive retention, nullable usage, per-call timing, versioned USD estimates and implementation acceptance checks.
- Selected design: [B Side inspector with failure examples](design/directions/round-2.html). Status follows the timestamp; error reasons appear in the entry metadata section. The existing Handy palette and navigation are retained; request snapshots and all metrics in the samples are synthetic.
- Evidence: `node scripts/verify-history-refinement.mjs` passes for header placement, five error cases in the details section, request/call inspection, compact dark/light layouts and focus restoration. Static samples and the verification record are in `design/directions/proof/round-2/`. The original three-option comparison is retained in round 1.

## History implementation task graph

Working directory: `D:/rust/Handy`. Source: [History specification](design/components/history-post-processing.md) and [selected Option B behavior](design/directions/round-2.md). The user approved implementation on 2026-09-27, requested a GPT-6-Sol implementation subagent, and assigned the parent agent review. All existing working-tree changes must be preserved. The accepted spec's local archive default and bounded retention are included. Unknown model prices remain unavailable; fictional design rates must never become production rates. No new profile replay behavior is authorized.

### H10 — Persist observed processing runs and provider calls

- Status: done
- Depends on: none
- Mode: AFK
- Files: `src-tauri/src/managers/history.rs`, `src-tauri/src/llm_client.rs`, `src-tauri/src/actions.rs`, `src-tauri/src/context_profiles/request.rs`, `src-tauri/src/commands/history.rs`, related Rust modules/tests and bindings.
- Source: History specification sections Current code and gaps, Usage/duration/cost, Persistence and integration, Acceptance.
- Acceptance:
  - [x] Preserve version-4 history through additive migrations; expose lightweight run summaries and lazy call detail.
  - [x] Record successful/failed/cancelled/interrupted runs and every actual probe/rewrite/retry with immutable provider/model/profile snapshots, optional reported usage, monotonic timing and accurate output outcome.
  - [x] Preserve observations on HTTP/parse/validation errors and cancellation; never revive deleted entries or overwrite earlier run evidence.
  - [x] Retain exact safe request bodies with bounded size, credential exclusions and explicit archive states; cleanup cascades work on every connection.
- Verification: targeted Rust migration, local HTTP fixture, lifecycle, usage and cleanup tests; review the diff against the pre-implementation snapshot.
- Evidence: Parent inspected the storage/client/live/retry changes against the pre-implementation snapshot and independently ran `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features --quiet`: 312 passed. Coverage includes a real version-4 upgrade, cascade/archive controls, interruption and deleted-ID isolation, a local HTTP reasoning-retry archive comparison, bounded usage/error classification, and blocked-output preservation. Parent fixed valid profile rewrites being misreported as processing failures and the legacy prompt field bypassing archive-off. Native UI verification is H12; user-facing metadata-write failure notice is tracked with H11.

### H11 — Deliver the selected History inspector and cost/archive controls

- Status: done
- Depends on: H10
- Mode: AFK
- Files: `src/components/settings/history/`, `src-tauri/src/settings.rs`, history commands/pricing modules, `src/bindings.ts`, locales, related frontend/backend tests.
- Source: History specification and selected Option B mockups; settled user requirement that failure reasons appear in the provider/token/cost section and status sits beside the timestamp.
- Acceptance:
  - [x] Implement responsive side inspector with Overview/Requests/Calls, original/processed copies, older-run inspection, visible error reasons and focus/keyboard behavior.
  - [x] Add request-archive setting and clear-archive control, preserve metadata when disabled/cleared, and never request the provider while inspecting history.
  - [x] Calculate estimates only from verified exact-model rate snapshots with decimal precision; handle unknown/partial usage and prices explicitly.
  - [x] Localize new UI strings and retain existing audio/history actions without enabling profile replay.
- Verification: frontend build/lint/translation checks, browser fixtures at 680 x 570 and larger in both themes, price/archive tests.
- Evidence: Generated Specta commands/types are used by the frontend. Parent independently passed frontend build, lint, translation-key coverage, nine decimal/partial-usage helper assertions, and four final Edge inspector tests covering compact/wide layouts, light/dark themes, keyboard tabs, clearing previously viewed archives and delayed requests across run switches. Existing profile/overlay regression tests also passed (23 tests). Native verification confirmed the status beside the time and the error inside the metadata block below audio. Initial estimates are limited to exact direct-OpenAI Standard `gpt-6-luna` with complete usage categories; rates and sources are recorded in `design/history-pricing-sources.md`. New locale text is English pending translation.

### H12 — Independent review and native verification

- Status: done
- Depends on: H10, H11
- Mode: AFK
- Files: review fixes in H10/H11 files, focused tests/native proof scripts, `plan.md`, implementation status in the History specification.
- Source: all History acceptance criteria and user request for parent review.
- Acceptance:
  - [x] Parent inspects changed code and independently verifies critical migration, request snapshot, unknown usage, error, cancellation and output behavior.
  - [x] Rebuild and launch the owned dev app with preserved data; verify real native command persistence and History rendering using isolated test entries and a local endpoint where practical.
  - [x] Record precise verification results and any remaining live-speech/platform limitations; do not equate mock browser tests with native end-to-end proof.
- Verification: relevant Rust and browser suites, frontend/native builds, lint/translation/format checks and a native History walkthrough/proof.
- Evidence: Parent reviewed against the pre-implementation snapshot, fixed blocked output being classified as LLM failure, prevented late legacy-template writes from undoing archive clearing, and required retry transcript updates to survive metadata failures. Final Rust suite: 313 passed; native and frontend builds pass. Touched-source Rust/Prettier checks pass. `node scripts/verify-history-native.mjs` passed on the rebuilt Windows ARM64 dev app using synthetic speech and four local HTTP attempts: success, 429 rejection, archive disabled, and malformed HTTP 200 with retained usage. It verified exact request contents, immutable attempts, native UI updates, inspector navigation without provider calls, Escape/focus restoration, cleanup and settings restoration. Evidence: `design/proof/history-native/result.json`, `failure-entry.png`, `request-inspector.png`. The app remains running with preserved portable data. No paid provider call, live microphone recording, macOS/Linux native run or screen-reader session was performed in this verification; those limits are also recorded in the History specification. Existing Rust warnings and the frontend bundle-size warning remain.

## Processing-message implementation task graph

Status: Reviewed and ready for implementation; implementation has not started. Working directory: `D:/rust/Handy`. Source: [Processing messages specification](design/components/processing-messages.md), [implementation documentation](design/implementation/processing-messages.md) and [review decisions](design/processing-messages-review.md). Commands below run from this working directory with the existing Windows build environment described in the [verification document](design/implementation/processing-messages-verification.md).

Settled scope: Three contextual request types; the selected profile owns its system prompt and dictionary. User messages carry task guidance, captured field text, memory and dictation. General remains a routing fallback. This slice includes prompt migration, exact selection ranges, safe output guards and History provenance. It does not add browser/terminal capture adapters, model hosting, automatic learning, empty replacement/deletion, conversation replay or a new output coordinator. Unknown selection can produce a candidate but cannot authorize automatic paste. Preserve the explicit auto-submit preference; profile output bypasses trailing-space modification.

Execute the tasks in order. They share profile/session types, bindings, actions and tests, so implementation parallelism is not assumed. Documentation authors may work on disjoint files; this does not make code edits independent. Do not release an intermediate build with only part of the new capture/classification/output contract. Every implementation task starts pending and requires new verification evidence; the earlier feature's passing tests do not establish this design's correctness.

### PM01 — Own and migrate profile prompts

- Status: pending
- Depends on: none
- Mode: AFK
- Files: `src-tauri/src/context_profiles/storage.rs`, `src-tauri/src/context_profiles/routing.rs`, `src-tauri/src/context_profiles/session.rs`, `src-tauri/src/context_profiles/template.rs`, `src-tauri/src/context_profiles/request.rs`; `src/components/settings/context-profiles/`; `src/i18n/locales/*/translation.json`; `src/bindings.ts`; `tests/context-profiles.spec.ts`.
- Source: Processing messages specification sections Existing profiles and prompt configuration, System prompt; [migration implementation document](design/implementation/processing-messages-migration.md).
- Acceptance:
  - [ ] Catalog v1 becomes v2 without losing profile IDs, routing, dictionaries, icons or custom prompt prose. Effective inherited prompts become independent owned copies.
  - [ ] Only exact known shipped seed text converts automatically. Incompatible custom prompts remain stored and editable with an explicit review state; processing is blocked until a valid prompt is saved.
  - [ ] New profiles own a generic default; prompt modal Save/Cancel/default actions and dictionary-only placeholder controls work without inheritance. Ordinary profile autosave remains usable for review-required profiles.
  - [ ] Migration is idempotent, failure preserves the previous store/cache, and new prompt readiness is enforced before provider work.
  - [ ] Carry typed prompt readiness through frozen resolution to the request gate; do not turn a review-required profile into a generic unresolved-context timeout.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features context_profiles`; `PLAYWRIGHT_CHANNEL=msedge bun run test:playwright tests/context-profiles.spec.ts` (set the variable using PowerShell syntax on Windows). Add migration/review-modal cases; expected results are lossless migration and zero provider calls for review-required prompts.
- Evidence: pending.

### PM02 — Capture verified field and selection ranges

- Status: pending
- Depends on: PM01
- Mode: AFK
- Files: `src-tauri/src/context_profiles/session.rs`, `src-tauri/src/context_profiles/capture.rs`, `src-tauri/src/context_profiles/routing.rs` test fixtures; existing native capture fixtures and focused range tests.
- Source: Processing messages specification sections Request selection, Conversation context and markers, Incomplete context; [runtime implementation document](design/implementation/processing-messages-runtime.md).
- Acceptance:
  - [ ] Capture distinguishes verified no selection, verified absolute UTF-16 range, known unverified selection and unknown selection presence.
  - [ ] Full versus prefix extent is explicit. UTF-16 offsets convert safely and identify repeated occurrences without text search; selected substrings match the captured range.
  - [ ] Preserve 250 ms/4096-scalar/2048-scalar limits and protected-field exclusions. Unsupported controls remain unavailable. Selection-only context requires independent range proof.
  - [ ] Native fixtures cover empty/full-field selection, cursor at end/middle, repeated text, Unicode and oversized/unverified selection without clipboard capture.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features context_profiles::capture` and new focused range tests; inspect fixture evidence that no range is synthesized for an unsupported control.
- Evidence: pending.

### PM03 — Assemble contextual messages and validate one task classification

- Status: pending
- Depends on: PM02
- Mode: AFK
- Files: `src-tauri/src/context_profiles/request.rs`, `src-tauri/src/context_profiles/template.rs`, proposed pure message assembly module and `src-tauri/src/context_profiles/mod.rs`; request fixtures; related profile error translations where exposed.
- Source: Processing messages specification sections System prompt, Request templates, Subsequent rounds and responses; [runtime implementation document](design/implementation/processing-messages-runtime.md).
- Acceptance:
  - [ ] System content contains only the selected profile's configured prompt/dictionary and the fixed contract, with deterministic serialization. Changing user context or memory leaves it byte-identical.
  - [ ] One prepared classification chooses New/Continue/Edit or explicit degraded context and carries expected operation and capture proof to consumers.
  - [ ] Render readable messages, collision-free section/cursor/selection delimiters, truthful partial context and literal captured placeholders. Reject oversize/protected/review-required requests before compatibility or rewrite calls.
  - [ ] Retain structured response validation and synthetic compatibility checks, remove JSON-envelope assumptions, and reject returned operations that disagree with the prepared task.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features context_profiles::request` plus proposed assembly tests; a local HTTP fixture compares exact sent bodies for three types and degraded requests and counts zero calls for local rejection. This slice verifies assembly and the HTTP boundary; PM05 must repeat the dispatch checks with migrated History storage before the integrated contract is considered verified.
- Evidence: pending.

### PM04 — Dispatch exact fragments only with valid output proof

- Status: pending
- Depends on: PM03
- Mode: AFK
- Files: `src-tauri/src/context_profiles/request.rs`, `src-tauri/src/context_profiles/session.rs`, `src-tauri/src/actions.rs`, `src-tauri/src/clipboard.rs`, relevant `src-tauri/src/paste_tx/` hooks and output tests.
- Source: Processing messages specification sections Incomplete context, Subsequent rounds and responses; [runtime implementation document](design/implementation/processing-messages-runtime.md).
- Acceptance:
  - [ ] Revalidate target, range, caret and relevant text after configured pre-paste delay immediately before dispatch; block stale or unverifiable output and retain the candidate.
  - [ ] Unknown selection produces no automatic paste; verified no selection with missing field context requires fresh no-selection/focus proof. Known unverified selection does not reach the provider.
  - [ ] Profile output bypasses trailing-space addition. Preserve ordinary output behavior and the explicit auto-submit preference. A blocked dispatch never auto-submits.
  - [ ] Existing paste methods use the required guard at their actual dispatch boundary; methods unable to honor it fail recoverably. Clipboard cleanup still occurs after guard rejection. Do not introduce a second coordinator or claim atomic external-field locking.
  - [ ] Reliable paste pays the configured delay before publication, then checks immediately before injection. A typed guard rejection reaches History as blocked, restores the owned prior clipboard even under CopyToClipboard, suppresses submit and never retries through legacy fallback.
- Verification: focused request/input-unchanged tests and existing clipboard/paste transaction tests, plus controlled native fixture changes during the configured delay. Expected result: wrong occurrence/cursor changes never pass the guard and returned profile fragments remain exact.
- Evidence: pending.

### PM05 — Preserve message provenance in History

- Status: pending
- Depends on: PM04
- Mode: AFK
- Files: `src-tauri/src/managers/history.rs`, `src-tauri/src/managers/history_processing.rs`, `src-tauri/src/commands/history.rs`, `src-tauri/src/actions.rs`; `src-tauri/src/context_profiles/request.rs`; `src/components/settings/history/`; locales; `src/bindings.ts`; `tests/history-inspector.spec.ts`.
- Source: Processing messages specification section Subsequent rounds and responses; [migration implementation document](design/implementation/processing-messages-migration.md).
- Acceptance:
  - [ ] Add a new migration after v5 with nullable request type, context mode, request-template version and prompt/dictionary revisions; retain legacy values and IDs.
  - [ ] Attempt provenance writes before actual calls and retain applicable provenance on preflight failures without inventing provider attempts. Preserve best-effort processing on storage failure with a visible incomplete-details warning; never use persistence as output authority.
  - [ ] History distinguishes the three request types, context-free candidates, blocked output and older rows with no metadata. Show owned versus historical inherited provenance truthfully.
  - [ ] Exact request archives match sent messages. Archive-disabled/clear behavior removes content while preserving non-content provenance, usage and existing lifecycle protections.
  - [ ] Start profile runs without ordinary prompt text/source, retain intended template version, and keep pre-resolution failures ineligible for ordinary History replay.
  - [ ] Persist per-run archive authority in migration 6. Clear/opt-out revokes late writes from existing runs transactionally; new runs follow the current preference. Paused-run tests prove templates and later calls cannot recreate cleared content.
- Verification: history migration/storage tests and `PLAYWRIGHT_CHANNEL=msedge bun run test:playwright tests/history-inspector.spec.ts`; local HTTP assertions compare retained messages and ensure inspection creates no calls.
- Evidence: pending.

### PM06 — Verify the complete migrated native pipeline

- Status: pending
- Depends on: PM05
- Mode: AFK
- Files: proposed `src-tauri/examples/processing_messages_fixture.rs`, proposed `scripts/verify-processing-messages-native.mjs`, a proposed development-only fixture command with registration in `src-tauri/src/lib.rs` and reused session/action helpers, focused tests and generated bindings as needed; `design/proof/processing-messages/`, this graph and implementation verification documentation. Proposed files do not yet exist.
- Source: Processing messages specification Acceptance criteria; [verification implementation document](design/implementation/processing-messages-verification.md).
- Acceptance:
  - [ ] Run full Rust/frontend checks and relevant profile/overlay/history browser regressions. Verify migrated settings and prompt modal behavior across restart.
  - [ ] Use supported native Edit fixtures and a local provider to prove New, Continue end/middle, Edit repeated/full-field text, later recordings, archive behavior and exact output.
  - [ ] Build a foreground Edit helper and explicitly gated development-only synthetic-audio entry point into the real session pipeline. Verify actual focused HWND, prohibit the fixture command in release builds, and do not substitute the offscreen adapter fixture or History retry for live target/output proof.
  - [ ] Prove blocked late changes, unknown selection, local validation rejection, unsupported control handling and unchanged ordinary output; cover trailing-space and auto-submit combinations.
  - [ ] Restore settings, remove only owned fixtures and leave the user's data intact. Record actual platform/control coverage, residual OS race limits and any remaining failures. No paid endpoint is required.
  - [ ] Restore nullable prompt selection through the documented loaded-store fallback when the public command cannot clear it, verify all restored settings after restart, and report success only after cleanup.
- Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked --no-default-features`; `bun run build`; `bun run lint`; `bun run check:translations`; relevant Playwright suites with installed Edge; proposed native proof script. Passing browser fixtures alone do not satisfy native acceptance.
- Evidence: pending.


## Repository checkpoint (2026-10-01)

The feature branch checkpoint includes context profiles, verified correction memory, processing history, design/specification work, and reproducible experiment scripts. PM01-PM06 and the outstanding T09 desktop review remain pending as recorded above. Validation: 322 Rust tests; 28 profile, overlay, and history browser tests; frontend and native builds; ESLint; Rust formatting; Clippy (12 warnings, no errors); all 25 translated locales have the required keys. Local credentials, model/runtime downloads, experiment results, and portable app data are ignored. A scan of the 356 candidate files found no credential patterns or copies of the configured experiment API key.
