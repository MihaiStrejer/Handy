# Context profiles: intent

Status: Original intent, preserved as historical context. The implemented memory contract in [spec.md](spec.md) supersedes the structured dictionary and effect requirements below. This records the agreed direction and open design questions, not implemented behavior. The [specification](spec.md) contains the UI and integration contract; the [implementation plan](plan.md) maps that contract to the current code.

## Goal and boundary

Build Handy's context layer around the existing post-processing path. The forward path captures the destination application's useful context, resolves a profile and dictionary, tells the user which profile and edit state are active, and assembles the transcript and context up to request handoff. A feedback hook then accepts a validated action from the post-processing result so either a new misheard form under an existing keyword or a conversation-specific short-term memory can be included in the next request for that profile. Handy will not host or manage a text model for this feature. Context-aware requests always use Handy's configured post-processing endpoint.

The working name is **Context profiles**. A profile has three parts: Context selection chooses the rules for gathering and matching context and its system prompt template; Long-term dictionary holds canonical keywords as keys, each with a list of phrases that speech recognition has mistaken for that keyword; Short-term memory holds recent conversation-specific corrections for the next request. Handy will support multiple profiles. Profiles may be routed by application, workspace, or another signal that an application-specific resolver can obtain reliably.

Post-processing keeps its existing `post_process_enabled` flag. A separate `post_process_profiles` flag controls the context layer and can be enabled only while post-processing is enabled. Handy shows its existing Post-processing tab when `post_process_enabled` is true and the Context profiles tab only when both flags are true. Disabling post-processing clears `post_process_profiles`. The selected provider stays in Post Process. General owns the default system prompt template for profile mode; a specific profile can override it, or inherit it when no prompt is defined for that profile.

## Voice-session flow

1. **Capture before focus changes.** At voice-session start, capture the destination application and whatever the focused input exposes: selected text, caret, and bounded surrounding text. Record whether each value is present, empty, unavailable, or uncertain. Protected input content must not be included.
2. **Resolve a profile.** Run the resolver appropriate to the application and apply matching rules from specific to general, then snapshot the chosen profile, dictionary, and effective system prompt template for this session. If a workspace is unavailable, continue to the application rule and then General without a separate per-profile fallback setting. Do not silently change that choice if focus, application context, or prompt settings change while recording.
3. **Show the choice in the recording widget.** Replace the pink dot at the far left of the recording pill with the identified profile's icon as soon as the profile is known. Use that existing slot in both the compact and Live widget. Preserve microphone readiness feedback; identifying a profile does not mean recording has started. Provide the exact profile name through a tooltip and accessible label, with a short visible name where space permits. The full workspace path stays internal to routing. Show a distinct edit indicator when a nonempty selection is being replaced, and a compose state when no text is selected. If context is incomplete, show a fallback or unknown state rather than implying a specific profile or edit target was verified.
4. **Transcribe and assemble.** Use Handy's existing audio and transcription flow. Assemble the transcript, effective profile system prompt, dictionary and memory snapshots, and only the captured context permitted for the endpoint. Resolve supported placeholders from the frozen session data. Keep spoken text, dictionary data, and text read from another application distinguishable from instructions.
5. **Hand off and receive feedback.** Send the assembled request into Handy's existing post-processing path. That path remains responsible for calling the configured endpoint and handling insertion. If its completed result identifies a validated misunderstanding of an existing keyword, append the misheard phrase to that keyword's values in the originating profile's long-term dictionary. A validated conversation-specific correction instead enters that profile's short-term memory for its next request.

The widget state and the request must refer to the same session snapshot. The widget must not expose selected text, field contents, or other private context merely to explain the profile choice.

## Application-specific context

Application identity is the first routing signal, but it may be too broad. A terminal window can host a shell, Codex, Claude Code, or another harness; the useful profile may be determined by the **workspace directory** rather than the harness name. For example, a terminal session associated with `D:\work` could route to the `D:\work` profile and its dictionary, regardless of which harness is running there.

A terminal resolver should report the workspace only when it can establish which session or tab owns the focused input and which directory belongs to that session. Window titles or a nearby process are possible clues, not proof. If the directory cannot be established, continue matching by application and then General; show the profile actually chosen in the widget. Other applications may need different resolvers and matching signals; the first supported applications and signals remain to be chosen.

Selected text is another context dimension. A verified nonempty selection places the session in **edit** mode: the request includes the selected text and available surrounding text so the endpoint can rewrite the selected span. No selection places the session in **compose** mode: the request asks for new text at the intended caret. The mode describes the user's current input state; it does not by itself prove that a dictionary entry is wrong.

## Request handoff

The request is a conceptual contract, not yet a fixed Rust type or wire format. It should carry:

| Field                | Purpose                                                                                                                                                                                                            |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Transcript           | Handy's speech-recognition output for this session.                                                                                                                                                                |
| Profile              | Stable profile identity, match basis, whether routing used a fallback, and effective prompt source.                                                                                                                |
| Long-term dictionary | Stable dictionary identity and revision snapshot. Each canonical keyword is a key with a list of observed misheard phrases as its values.                                                                          |
| Short-term memory    | A bounded snapshot of recent accepted conversation-specific corrections for this profile, kept separate from keyword aliases.                                                                                      |
| Input context        | Source application, verified workspace when available, edit or compose mode, selected text, bounded field excerpt, and explicit unavailable states. A private insertion target stays outside the endpoint payload. |
| Instructions         | General's default system prompt template or the selected profile's override, including the template revision and supported placeholders. The provider remains the one selected in Post Process.                    |

The context layer must not substitute clipboard contents for unavailable accessibility data. It must not present an inferred terminal directory or selection as verified. The user must be able to tell when captured field or workspace context will be sent to an external endpoint. The template editor should show whether a prompt is General's default, inherited from General, or overridden for one profile. An inherited profile follows future edits to General on its next session; an override remains independent.

## Response contract to design next

Each completed post-processing prediction should provide **the predicted text** and **an action description**. The action should distinguish the text operation, such as new insertion versus replacement of a selection, from a proposed dictionary alias or a conversation-specific memory. A replacement may mean the user changed their mind, or it may reveal a repeatable misheard keyword; replacement alone is insufficient evidence to update either store.

For example, an action could describe `replace_selection` with no learning effect, or propose that `codecks` is a misheard form of the existing keyword `Codex`. The model's action is a proposal until Handy validates the session, keyword, misheard phrase, and completed text operation. Handy then appends a new, non-conflicting phrase under that keyword; the endpoint never mutates storage directly. A correction about the current conversation, such as a changed meeting date, belongs in short-term memory and is fed into the next request for that profile. The exact action schema, collision handling, evidence requirements, and memory duration remain open.

## Existing boundaries

Handy's current post-processing selection and request are in [`actions.rs`](src-tauri/src/actions.rs) and [`llm_client.rs`](src-tauri/src/llm_client.rs). The recording widget is [`RecordingOverlay.tsx`](src/overlay/RecordingOverlay.tsx), and backend overlay events come from [`overlay.rs`](src-tauri/src/overlay.rs). The current [`custom_words` setting](src-tauri/src/settings.rs) is an ASR aid, not the proposed identity dictionary. These are integration points, not evidence that profile or input-context capture already exists.

## Open questions

1. Which application and workspace signals can be verified on each supported platform, especially when a terminal has several tabs or processes? Which applications and controls are in the first support set?
2. What profile matching precedence and manual override should apply when the application, workspace, and field suggest different profiles? Workspace failure itself uses the implicit application-then-General match order.
3. How much selected or surrounding text may be captured and sent, how is external transfer disclosed, and what should the widget show when capture is partial or unavailable?
4. How are canonical keyword IDs and their lists of misheard phrases stored, which source owns them, and how are duplicate or conflicting phrases resolved without changing the existing `custom_words` behavior?
5. What action schema separates insertion or replacement, a proposed keyword misunderstanding, and a conversation-specific memory? What evidence is sufficient to append a misheard phrase automatically?
6. Which of Handy's currently selectable post-processing providers can accept the full context request and return the required action description, and what should happen when the chosen provider cannot?
7. Which Mustache-style placeholders are supported in the first release, how are missing values rendered, and how is General's default initialized from Handy's existing Post Process prompt?
