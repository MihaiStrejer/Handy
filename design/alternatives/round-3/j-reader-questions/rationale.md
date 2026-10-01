# J · Questions a newcomer asks

## Editorial choice

This page starts with one compact map of the full session, then answers five questions in the order a newcomer is likely to ask them: what is ready, what happens during speech, what crosses the network, how output reaches the destination, and what remains for a later recording. The map keeps audio and context capture side by side until they meet at request assembly. It names real project objects such as profiles, transcripts and the configured endpoint, and labels its shapes as stages in a code path rather than new components.

The answers add detail beside that path instead of hiding facts behind a stepper or dropdown. Two embedded native settings captures explain editing and runtime routing; the native recording overlay ties the explanation to the visible recording state. Numbered cues sit over the images, with their meaning stated in nearby text. The opening diagram is an explanatory SVG derived from code, not an application capture.

The network answer shows the two request messages and all four data fields because “what leaves my PC?” needs concrete contents and a reason for each. It also names the authentication header, distinguishes the synthetic endpoint compatibility probe from a voice request, and corrects the ambiguous word “cache.” The output answer separates checks from dispatch and says explicitly that dispatch is not readback. The final answer distinguishes saved profile configuration from session data and process-local memory, then keeps F’s G1-A/B/C options and their pending status visible.

## Evidence used

- `src-tauri/src/actions.rs`, `src-tauri/src/lib.rs`, and `src-tauri/src/context_profiles/mod.rs` show application setup, post-process invocation, concurrent audio/context work, and context session setup.
- `src-tauri/src/context_profiles/storage.rs` and `routing.rs` show saved profile data, the in-memory profile catalog, process-local memory, app matching, and General fallback. `capture.rs` and `target.rs` show the current Windows capture boundary.
- `src-tauri/src/context_profiles/request.rs` shows request assembly, all four user data fields, protected-input handling, reply checks, and the synthetic compatibility probe. `session.rs` shows the frozen session snapshot and prediction lifetime. `llm_client.rs` shows message serialization, HTTP authentication, and endpoint response handling.
- The supplied captures are from 2026-09-27, branch `feat/context-profiles`, commit `8f9cf53`. They show actual native UI populated with synthetic profile values; the overlay was populated from a session-event fixture. They do not show a live voice recording or external endpoint request.

## Limits and wording decisions

The source map does not verify a complete microphone-to-paste run, provider caching behavior, or destination readback. The page makes no such claim. It describes supported selection as native Windows Edit only; app-based routing can still match an executable without reading its selected text. Windows Terminal workspace matching remains deferred. The request’s provider and model are read while processing, so the diagram does not imply that recording start chooses them.

The process-local memory map can be included in later requests, but the inspected code has no writer that inserts a model-suggested correction. Therefore the page does not imply that a correction learned in one session is available in another. G1 remains open, and G1-A is labeled as a recommendation only.

## Verification

The parent opened the standalone page in installed Edge at desktop and mobile widths with JavaScript disabled. It confirmed that the overview branches into audio and context capture, both handoffs reach request assembly, labels fit, G1 options remain readable, and the mobile diagram has a visible scroll hint. The offline page made no external requests. I also checked that all nine linked source files exist, all three embedded images have PNG signatures, no image placeholders remain, and the page has no duplicate IDs or external assets. Reader comprehension and a live application voice session remain unverified.

## Transfer example

For a hypothetical photo backup service, a newcomer page could map: account setup → scan local folders → upload selected originals and metadata → confirm remote receipt → apply the next scheduled scan. Questions could then explain which folder rules persist, what files leave the device, what upload success means, and whether deleted files affect the next scan. The same approach follows the reader’s real questions and connects evidence to relationships without requiring Handy’s profiles, audio paths, network messages, or a fixed number of sections.
