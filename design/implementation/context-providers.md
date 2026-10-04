# Application context providers

Status: implemented and verified on 2026-10-04. Started-executable proof is recorded below; microphone and real-model memory verification remain separate MR09 gates.

## Agreed scope

The context builder needs extendable application-specific extraction providers. Ship T3 Code and Windows Terminal/PowerShell first. Every other application uses the `default` provider and resolves to the default/General profile. This supersedes unrestricted application-rule routing for contexts without a supported extraction provider.

Extraction providers and post-processing HTTP providers are separate concepts. An extraction provider describes how to obtain local application context. A processing profile owns the prompt and memories used with that context. The settings page offers editable Project directory, T3 Code and Windows Terminal / PowerShell templates. Templates are added on demand, preserving existing profiles. The optional preference question received no reply; on-demand creation follows the recommended assumption, not an explicit user selection.

The user clarified that the working directory is the main context switch. A verified active working directory is therefore the primary project-profile routing key, independent of which supported application extracted it. The same directory in T3 Code and Terminal should select the same project profile. Provider identity selects the extraction method; project/conversation/branch labels enrich that profile's context. A directory rule must not require the same application executable to match as well. Use exact normalized directory matching with deterministic ambiguity handling. If no verified directory rule matches, retain the supported provider's applicable fallback; unknown/default-provider contexts always resolve to General. Display labels and guessed directories cannot select a project profile.

## Observed running applications

Read-only native proof is outside the repository at `D:/Tools/Handy-qa/2026-10-04_feat-context-profiles_t3-capture/index.md`. Reviewer verdicts remain pending.

- The actual production `target.rs` and `capture.rs` ran in an isolated harness against the focused T3 Message composer. Application capture succeeded; workspace, selection, surrounding text and caret were unavailable. This is a production-code probe, not a started Handy GUI or microphone test.
- T3 initially exposed only its window shell. An ordinary MSAA accessibility-object request exposed its full UI Automation tree without restarting or modifying T3. The tree included project `prx-ascend-docs`, conversation `Calendar Line Color Picker`, branch `dev` and the Message composer.
- The composer exposes ValuePattern, TextPattern and a runtime ID. Its UIA element has no native HWND; its containing focused HWND is `Chrome_RenderWidgetHostHWND`. Handy currently accepts only a native `Edit` HWND for field capture.
- Windows Terminal has more than one window in one process. Querying a process's first MainWindowHandle is therefore insufficient. The `Windows PowerShell` window exposes a selected TabItem and a TermControl with TextPattern. Another window in the same process has a different active tab title.
- The initial T3/PowerShell inspection established no verified current directory. Window/tab titles and project labels are advisory text, not workspace-path evidence.
- No draft content was saved, typed or submitted. Draft equality before/after T3 focus inspection was true. No clipboard, microphone, user-store mutation or model request was used.

A follow-up explicitly requested inspection of the open Claude Code terminal. Its running Claude process and parent PowerShell both expose current directory `E:\Steam\steamapps\common\DarkestDungeon\` through a bounded, stable, read-only native process-parameter lookup. The selected Claude terminal tab's TextPattern independently contains that exact directory. Proof is at `D:/Tools/Handy-qa/2026-10-04_feat-context-profiles_claude-cwd/index.md`. No commands were entered and no full terminal transcript or process environment was saved. This establishes that the CWD is obtainable for this process; the implemented helper now binds one owned console to the captured window and rechecks the selected tab. Multiple consoles with the same connected title or ambiguous clients return unavailable.

## Provider contract

Use one registry with explicit matching and extraction implementations. A new provider registers its application matcher and extractor against the same bounded result contract; it must not add application-specific branches to profile routing, request assembly or memory admission.

Resolve a provider against the frozen native target, not whichever application becomes focused when an asynchronous lookup starts. Match exact known application identities and supported window/control classes. The default provider is the final fallback. A known provider with unavailable metadata remains known; metadata failure must not be confused with an unknown application.

The result identifies the extractor and carries separately typed availability for window title, project display name, conversation, branch, active terminal tab, focused input metadata and verified workspace path. Optional metadata is reference context, never instructions. Include provenance and bounded/truncated status; omit opaque native/UIA identities from endpoint payloads and ordinary events/logs.

Preserve the existing one-worker and deadline limits. Use targeted queries and bounded metadata, not a complete chat/scrollback traversal. The diagnostic T3 traversal took about 588 ms and exceeded Handy's 250 ms budget; it is evidence of availability, not a suitable production algorithm. Cold accessibility initialization may yield a truthful timeout. Late results cannot attach to newer sessions.

### T3 Code

Recognize stable and Nightly application identities. Initialize/query standard accessibility when needed. Establish the current project/conversation through the visible breadcrumb and active thread evidence, rather than selecting the first matching sidebar label. Obtain branch metadata from its current control; a `dev` mention in conversation content is not branch evidence.

Capture composer identity, role, editability and supported patterns. A project label is not a filesystem path. Unknown/unavailable fields remain explicit. Metadata capture alone does not authorize selection replacement or short-term learning.

Verified composer text/selection capture must bind the same UIA element runtime identity to the frozen session and repeat that identity check during preflight and readback. A browser HWND shared by multiple inputs is insufficient. Keep the existing exact-field/range/caret checks; do not enable inferred replacement or memory admission merely because TextPattern is present. Implement or verify this integration separately from metadata when necessary.

### Windows Terminal / PowerShell

Extract from the captured window and its selected tab, not a neighboring tab, another window in the process or the first shell process. Use terminal accessibility APIs separately from T3's breadcrumb/composer logic. Distinguish the host, tab title and shell display label from verified shell identity.

A directory is eligible for workspace routing only when a provider can establish it for that active terminal context. Title parsing, visible prompt text and a parent/sibling process's directory are insufficient. The provider requires one visible TermControl and reads its connected-console title from UIA HelpText, independently of the editable tab caption. The disposable helper checks console root-owner against the captured window, requires exactly one owned console with that connected title and one leaf client, and reads that client's stable x64 process CWD. It rechecks the console title; the provider rechecks pane identity, connected title and selected tab before publishing the directory. Other tabs with different console titles do not prevent capture. Duplicate console titles, splits, ambiguous clients, unsupported architecture, denied process access or timeout return unavailable; application fallback remains available. Handy does not attach its own process to the observed console. The helper duplicates its IPC stdout before attaching and writes only to that pipe after detaching.

The Claude probe demonstrates a possible native process-CWD reader. It must be paired with verified terminal context ownership, not a scan that picks the first `claude.exe` or PowerShell process. Native structures are architecture/version-dependent and require bounded validation and a truthful unavailable fallback; [Microsoft documents this constraint](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntqueryinformationprocess). This discovery does not authorize shell input or an unverified path match.

Terminal TextPattern commonly covers a screen buffer, not an editable input field. Do not treat scrollback as the current input or invent caret/selection boundaries. No shell commands or keystrokes are needed for metadata extraction.

### Default

Keep basic application identity and truthful generic availability. Resolve to General even if a legacy application rule would otherwise match an unsupported application. Generic native Edit capture can remain available, but it does not turn that application into a dedicated provider. Ambiguous provider matches must have a deterministic documented fallback.

## Integration and verification

Freeze provider metadata into the recording's existing InputContext and carry it into the context builder's reference payload. Keep local identities out of serialized data. Preserve profile revision, cancellation and memory-authority checks. Processing-message tasks remain separate and consume this enriched snapshot when implemented.

Update the existing routing model accordingly: workspace rules now ignore the executable field, expressing one directory-owned profile shared by T3 and Terminal. Directory-first routing preserves existing profiles and requires verified evidence. Preserve exact normalized path comparisons; prefix matching must not route a neighboring directory to the same profile.

Verify actual T3 and Terminal metadata, two-window/two-tab isolation, unknown application -> default -> General, cold accessibility/deadline behavior, provider failure, protected fields, focus changes and session expiry. Run existing native Edit, routing, request, feedback and browser regressions. A production-code harness cannot substitute for a final started-app check; record both scopes honestly.

## Implemented owners and proof

- `providers.rs` owns the extractor trait, ordered registry, bounded typed metadata and exact absolute-directory normalization. `routing.rs` selects directory rules before supported application fallbacks; ties select General. Default-provider contexts always select General.
- `provider_windows.rs` performs targeted T3 breadcrumb, composer capability and branch queries and Terminal selected-tab queries. `terminal_process.rs` owns disposable console inspection and bounded stable x64 CWD reads. `capture.rs` preserves the 250 ms budget and one-worker limit; late results cannot attach. A 180 ms helper deadline leaves time for focus/tab rechecks.
- `ContextProfilesSettings.tsx` offers on-demand templates and editable directory rules. Metadata autosave retains existing long-term memory. No persisted schema change is needed for the existing nullable workspace rule.
- `handy --inspect-context` captures the foreground target before initialization, prints bounded context and the selected profile as JSON, and exits without microphone or model initialization. It runs independently of the installed single instance. Use an isolated portable catalog for QA; this command loads the profile store using normal initialization/migration rules.

Started-executable proof: `D:/Tools/Handy-qa/2026-10-04_feat-context-profiles_started-app/index.md` (three objectives, all human verdicts pending). The actual built Handy captured T3 project `prx-ascend-docs`, conversation `Calendar Line Color Picker`, branch `dev` and editable Message composer metadata without changing its draft. It captured the Claude terminal CWD and selected the synthetic Darkest Dungeon project profile by directory. An owned unsupported PowerShell Forms window selected General despite a matching legacy executable rule. The console helper separately isolated two windows in the same Terminal process; the other window returned unavailable rather than Claude's CWD.

T3's current accessible breadcrumb exposes a name, not a verified path. Its observed active project does not match the local T3 state database, so no database-derived path is used. T3 therefore uses an application fallback until a structured workspace integration supplies verified local/remote paths. T3 browser fields and Terminal screen buffers remain metadata only: no exact-field replacement or short-term learning is claimed. Dedicated extraction is Windows-only; other platforms truthfully use default/General.

Final regression checks: 351 Rust tests passed, one fixture evaluator intentionally ignored; 38 Playwright tests passed in Chrome; frontend build, ESLint, translation coverage (714 keys / 25 translated locales), Clippy and Rust formatting passed. Native debug build used `TRANSCRIBE_CMAKE_ARGS=-DTRANSCRIBE_VULKAN=OFF -DGGML_CPU_ALL_VARIANTS=OFF` because this machine has no Vulkan SDK. Production acceleration settings are unchanged. Global formatting reports 305 pre-existing checkout failures, with no changed path among them; changed-file formatting and whitespace checks pass. A failed enrichment stability check discards partial metadata and workspace evidence.

## Multi-tab routing correction

User testing found that a correct `3DMouse` directory rule fell back to General when the same Terminal window also contained the Handy tab. The original helper rejected all windows with multiple owned consoles. The correction associates the unique visible pane with its connected console, preserving owner, title, process-directory and stability checks. Native started-executable inspection against the active user terminal selected the isolated synthetic `3DMouse` profile by `D:\3DMouseShell\` in three consecutive captures. It read no terminal contents and changed no user settings. Evidence: `D:/Tools/Handy-builds/3dmouse-routing-diagnosis/fixed-context-1.json` through `fixed-context-3.json`, launched by `verify-fixed.ps1`. This verifies capture and routing; it does not claim a microphone or model run in the new binary.
