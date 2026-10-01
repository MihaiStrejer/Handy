# Alternative I: one illustrated session

## What this design is testing

This page asks whether a newcomer can understand Handy’s architecture by following one plausible session from saved configuration to the next recording. It keeps the whole flow in view near the top, then expands only the boundaries where a concrete example helps: runtime profile choice, parallel audio and context work, the two request messages, output checks, and learning after output. The example is Windows Terminal because the existing native captures show its application rule, dictionary and overlay icon.

The user says “Open Codex and review the changes,” and the illustrative transcription mishears it as “Open code desk and review the changes.” The captured dictionary contains “Codex” with “codecks” and “code X,” so “code desk” is an illustrative new mishearing. The sample reply proposes an additional misheard form to make G1 concrete. It is a possible reply under the parser contract, not a recorded endpoint result or proof that learning occurred.

## Composition decisions

The overview uses a wide sequence before the long story, with the capture and audio work placed together and labeled as independent. The chapters then use “Before,” “During” and “After” to preserve the reader’s sense of time. The settings screenshots appear beside the claim they support; the overlay appears beside the resolved session display. The request is the main technical closeup and shows both roles at readable size. G1 receives a full-width decision area with the three stable options from F, while the rest of F’s repeated detail table is omitted.

“Profile,” “session,” “system message” and “user message” are introduced by purpose before source symbols. The page avoids invented component names for editorial groupings. Source IDs S1–S11 point to files and line ranges, but those references are secondary to the narrative.

The session uses terminal selection as unavailable, which means the illustrative operation is insert. Windows native Edit selection is supported elsewhere; the page does not imply terminal or general browser selection is supported. The provider and model are read when processing starts. The frozen session data are the chosen profile, effective prompt, dictionary, memory snapshot, captured input and original destination, rather than a provider conversation.

## Evidence and limits

The images are embedded from design/proof/architecture/context-original.png, dictionary-annotated.png and overlay.png. Their capture.json records 2026-09-27, branch feat/context-profiles, baseline commit 8f9cf53cd1410cda26beea39ff802ac306e39585, isolated portable development data, synthetic profile values and a synthetic overlay event. The screenshots prove the pictured UI state, not hidden request behavior or destination insertion.

Code inspection covered src-tauri/src/lib.rs, actions.rs, context_profiles/mod.rs, request.rs, session.rs, routing.rs, storage.rs, capture.rs and llm_client.rs. The code supports the conditional profile path, the two-message request, synthetic endpoint compatibility probe, reply checks, session identity and final focus gate. A successful paste call is dispatch evidence only. The page does not claim a live voice run, automatic learning or provider cache benefit. The compatibility cache records a contract check, not prompt reuse.

G1-A remains the recommendation inherited from the prior decision presentation: read back a supported field before learning. G1-B asks for user confirmation; G1-C trusts dispatch. All three remain pending. Choosing G1-A would still require semantic correction checks and a policy for targets without readback.

## Transfer example

For an unrelated warehouse returns system, the same editorial method could follow one returned parcel from intake to inspection to refund, place actual screen captures beside the visible actions, show where inventory and payment work occur independently, and attach a failure path to the point where a refund can be blocked. The portable instruction should choose that representation from the reader’s question and evidence; it should not require voice sessions, model messages, caching or a fixed number of lanes.
