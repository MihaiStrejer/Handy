# Context profiles and profile memory

Current feature contract, reconciled 2026-10-03. Implementation details and limits are in [profile-memory.md](design/implementation/profile-memory.md); execution evidence is in [plan.md](plan.md#profile-memory-implementation-task-graph). Earlier mockups and architecture pages are historical schema-1 evidence.

## Profile setup and routing

Application extraction uses an extendable registry with T3 Code, Windows Terminal / PowerShell and default providers. Verified absolute working-directory rules select a project profile across supported providers, independently of executable; application rules are lower-priority fallbacks. Equal matches select General. Every unsupported application uses default and General. Settings offers on-demand editable templates and a primary working-directory field.

T3 supplies bounded active project/conversation/branch and composer capability metadata. Its current accessible UI does not supply a verified project path. Terminal CWD requires one console owned by the captured window, a corroborated selected tab and one unambiguous client with a stable native x64 directory read. Missing or ambiguous evidence remains unavailable. Browser composer metadata and terminal scrollback do not grant input replacement or learning authority. See [provider implementation and proof](design/implementation/context-providers.md).

Context profiles require post-processing to be enabled. Setup remains local: adding profiles, configuring executable/workspace rules, editing prompts, choosing icons and writing consolidation instructions require no model, API key or network call. Endpoint readiness is checked when processing a rewrite or explicitly consolidating memory. Ordinary post-processing and ASR `custom_words` remain separate.

Each profile has Context selection, Long-term memory and Short-term memory tabs. The sidebar selects the profile to edit; recording resolves its own profile from the captured application and verified workspace evidence. General is the routing fallback. Ambiguous or unavailable capture is reported truthfully. Native input handles, caret/range authority and output checks stay local.

General currently owns a prompt. Other profiles either inherit it or own an override. A recording freezes its effective prompt, routing result, permitted input context, long-term text and short-term records. Later settings edits cannot change that snapshot. The pending [processing-message design](design/components/processing-messages.md) changes prompt ownership and readable request templates in a separate PM delivery; it is not implemented by this memory rework.

## Memory ownership

Long-term memory is one persistent string per profile. It may contain a readable terminology dictionary, mistranscription or translation mappings, scoped guidance and other useful context. It is displayed as selectable read-only text. Ordinary profile saves use a narrow metadata DTO and cannot submit memory, revisions, receipts or Undo state.

Short-term memory is process-local and profile-local. It contains identified, revisioned correction notes with their exact evidence quote and local provenance. Users can remove individual notes or clear the list. Each profile keeps at most 20 notes and 6,000 Unicode characters, with at most 500 characters per note; older notes are evicted first. Notes disappear on restart, while long-term text persists.

Receiving a model response cannot write memory. Short-term admission requires an explicit direct correction grounded in the current transcript, a locally valid proposal, the originating session/profile authority and exact verified destination readback. Unsupported controls, stale sessions, cancelled output, changed focus or failed/unavailable readback add no notes. A valid text prediction can remain usable when its memory metadata is rejected. Changed minds, quotations, negated instructions and model guesses are not correction authority. The conservative local language checks do not prove universal semantic correctness.

## Rewrite protocol

The current system message contains the effective template and Handy's fixed submission contract. Supported template variables are `{{long_term_memory}}`, `{{short_term_memory}}`, `{{input_context}}` and `{{transcript}}`. One-pass rendering inserts references to fields in a JSON user envelope, not captured text into the system role. Unknown or malformed expressions fail before endpoint work. Reference data is untrusted and cannot redefine message roles or output authority.

The user envelope contains `long_term_memory` as a string, identified `short_term_memory` records, permitted `input_context` and `transcript`. The endpoint returns:

```json
{
  "text": "BOM",
  "operation": "replace_selection",
  "memory_changes": [
    {
      "action": "add",
      "text": "Use BOM when bomb refers to this term.",
      "evidence_quote": "not bomb, BOM",
      "target_id": null,
      "expected_revision": null,
      "wrong": "bomb",
      "corrected": "BOM",
      "scope": null
    }
  ]
}
```

This example requires a verified selected span and the literal transcript `not bomb, BOM`. Without a selection the operation must be `insert`. A submission contains at most four proposals. Replacing a note requires its known ID and revision; Handy assigns identities for new notes. Evidence must be quoted from the current transcript. Literal wrong/corrected terms and scope must be grounded in that quote. Known contradictory corrections require replacement of the existing record. Handy validates the whole batch, stages it, verifies output, then atomically admits it once. Repeated delivery does not apply it again.

Compatible endpoints use one forced `submit_rewrite` function, including tools-only responses with null content. It submits final text and proposals to the same local validator as JSON; it does not execute a second model turn. Multiple or unknown functions, refusals and truncated responses fail safely. No consolidation or long-term write function is exposed.

Capability discovery uses synthetic data only and tests tools independently of structured-output support. A definite unsupported-tool response permits one synthetic JSON fallback probe. Authentication, rate limiting, timeout and transient failures do not establish incompatibility. Successful capability results are cached by endpoint, model, credentials and protocol/mode; a warm rewrite makes one request. Actual private input is never replayed to discover capabilities. History archives actual serialized transport bodies under its existing opt-in controls, and records proposed actions separately from verified admission.

## Explicit long-term update

The Long-term memory tab has an editable instruction area populated with default consolidation instructions. The Update long-term memory button sits to its right. Clicking it starts an independent operation with a unique fixed system prompt and a user message containing those instructions, the existing long-term text and a frozen short-term snapshot. It uses the configured HTTP post-processing provider/model and has no recording or History request dependency.

The returned string is a complete replacement. The prompt requests preservation of useful existing knowledge, explicit scoped corrections, deduplication and visible unresolved distinctions. Local checks reject malformed, oversized or unexpectedly empty output; they cannot prove that a model preserved every fact. The operation applies only if its profile still exists, its long-term revision matches and its source snapshot was not destructively changed. New notes arriving afterward stay in short-term memory and do not silently enter the frozen request.

Commit persists before publishing the new catalog. Unrelated profile edits are merged without overwriting them. Cancellation and commit are serialized; whichever wins determines the visible result. Notes remain after consolidation. One Undo restores the previous long-term string with a new revision. Cancelled or interrupted pending work is not replayed on restart. Navigation/remount recovers operation status by profile and operation ID.

## Migration and bounds

Catalog schema 1 migrates directly to schema 3. Schema 2 was reserved by an older, unimplemented PM plan; it is not invented as a shipped migration input. The pending PM migration must target schema 4 from the actual schema-3 memory catalog.

Migration makes an exact, one-time `context-profiles.v1.backup.json` before changing the store. It converts each legacy term and alias into deterministic JSON-quoted long-term prose without a model, retaining literal Unicode, order, IDs in the backup, routing, icons and custom prompt prose. Parsed legacy memory placeholders are converted only at this boundary. Empty and oversized valid legacy memory survives without truncation. A failed backup or save retains the original store/cache authority.

Rewrite transcript/prediction bounds are 32,000 Unicode characters; submission arguments are at most 128,000 UTF-8 bytes. The user envelope is capped at 256 KiB and HTTP response bodies at 512 KiB. Consolidation instructions are 1?4,000 Unicode characters, consolidation input at most 256 KiB, and returned long-term text at most 32,000 Unicode characters. Provider work times out after 60 seconds. Oversized migrated content remains stored, but processing rejects requests that exceed its bounds.

## Verification limits

Automatic evidence covers the local contract, HTTP transport, owned Windows Edit capture/readback, persistence failures and mocked profile UI behavior. It must distinguish synthetic fixture results from real model quality and microphone/desktop behavior. See [validation evidence](design/profile-memory-validation.md) for commands, recorded results and remaining gaps. Prior T09 native review and pending PM tasks retain their own gates.
