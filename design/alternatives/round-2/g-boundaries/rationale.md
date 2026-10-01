# G · Boundary closeups inside an ownership map

## Design choice

The first screen makes the runtime owners visible before explaining the protocol. Three numbered boundaries connect the map to the two closeups below it: the session freezes context, the endpoint receives instructions and reference data, and output dispatch stops short of proving arrival. The reader can scan the whole path, then answer what crosses each boundary, why the split exists, what a claimed benefit depends on, and how the next step's status is known. No click is needed to expose the closeups.

The source page's responsibility lanes were useful, but its heading “Two messages per utterance” described the wire shape without explaining the purpose of the separation or the intended cache idea. This version distinguishes role authority from change frequency. The dictionary currently travels as user-role data even when it stays unchanged; the effective prompt travels as system instructions even when the user edits it. A proposed stable prefix is about matching content and endpoint support. It does not follow automatically from the system role. The current compatibility cache checks an endpoint with a synthetic request and stores no prompt computation.

The map keeps audio and context as parallel branches of one invocation, then joins them for the endpoint request. The page shows five causes and their consequences together rather than hiding them in interaction. The open G1 options remain secondary to the system explanation; G1-A is a recommendation, not an approved decision.

## Evidence and transfer

The implementation claims come from `src-tauri/src/context_profiles/request.rs`, `template.rs`, `session.rs`, `mod.rs`, `src-tauri/src/llm_client.rs`, and `src-tauri/src/actions.rs`. The page names current behavior, a proposal, and unverified end-to-end behavior separately. It is an offline diagram, not a screenshot or runtime test.

The same form could explain a shipping workflow: order entry, inventory reservation, carrier booking, and customer notice form the ownership map; a closeup at the reservation boundary shows the exact stock claim, why the inventory service owns it, what benefit depends on a committed reservation, and which status confirms or rejects it. If the reader instead needs to compare policy choices or operate a single screen, a decision table or annotated capture would serve better than this map.

## Review limits

I inspected the source paths and existing architecture template while making this alternative. I did not rerun the desktop app or prove a full microphone-to-destination flow. The page intentionally does not claim provider cache hits, reduced cost, an automatic learning writer, or selection capture outside supported native Edit controls.
