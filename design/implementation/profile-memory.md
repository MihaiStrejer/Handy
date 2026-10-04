# Profile memory rework

Status: Implementation authorized, 2026-10-02. See the canonical task graph for progress and verification. The canonical tasks are [MR01 through MR09 in plan.md](../../plan.md#profile-memory-implementation-task-graph). The working directory is the repository root, currently `D:/Tools/Handy`.

This delivery improves automatic correction learning, replaces the structured profile dictionary with free-text long-term memory, and adds a separate user-triggered consolidation process. It combines the two requested GPT-6.1-Sol analyses and the subsequent requirement to remove the old dictionary implementation.

## Requirements and proposed defaults

The user requested these behaviors:

- Post-processing can propose better short-term correction updates, potentially through tools.
- Long-term memory is a string that can contain terminology, incorrect transcription or translation patterns, and contextual guidance.
- A user initiates short-term-to-long-term consolidation with a button to the right of an editable instruction area. The area contains default instructions.
- Consolidation uses its own system prompt and the user's instructions. It runs independently of transcription.
- Normal model-driven correction learning cannot create or update long-term memory.
- Existing dictionary content is preserved during migration; the old dictionary implementation is cleaned up.

The following are recommended implementation defaults, rather than additional user requirements. They allow the task graph to proceed without introducing another approval step:

| Choice                                  | Proposed first delivery                                                                                                                        |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Rewrite protocol                        | Improve the validated JSON contract first, then add one native submission function on compatible endpoints.                                    |
| Short-term representation               | Keep text records with internal IDs, revisions and evidence; render their text as readable memory.                                             |
| Short-term lifetime                     | Preserve the current profile-local, process-lifetime behavior. No new persistence, TTL or conversation detector.                               |
| Explicit correction admission           | Keep the existing verified destination-readback requirement for every memory update, including explicit spoken corrections.                    |
| Inferred correction admission           | Require exact destination readback plus source grounding. New proposals initially target explicit corrections; inference remains conservative. |
| Long-term editing                       | Display long-term text read-only. Ordinary autosave edits consolidation instructions, not long-term memory.                                    |
| Consolidation model                     | Use the configured HTTP post-processing provider/model, captured when the button is pressed.                                                   |
| Consolidation result                    | A complete replacement string, applied after validation and a revision check. Keep one previous committed version for Undo.                    |
| Short-term handling after consolidation | Retain all records. New notes arriving during a request also remain.                                                                           |
| Dictionary compatibility                | Convert legacy fields and placeholders in migration. Provide no dictionary write API or runtime placeholder alias in the new schema.           |

Manual long-term editing, persistent short-term retention, non-Windows target adapters, append-only consolidation and additional model selectors remain possible later work. This delivery also excludes the companion audio API.

## Current implementation and source boundaries

The inspected baseline is `db21018` on `feat/context-profiles`. Source inspection establishes these facts; it does not establish runtime verification of the proposed delivery.

| Existing source                                                                                                                 | Current behavior                                                                                                                        | Integration                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| [storage.rs](../../src-tauri/src/context_profiles/storage.rs)                                                                   | Catalog schema 1; `Keyword`, `dictionary`, whole-profile autosave; short-term records live in memory.                                   | Versioned migration, text storage, narrow update commands, record revisions and batch admission.            |
| [routing.rs](../../src-tauri/src/context_profiles/routing.rs) and [session.rs](../../src-tauri/src/context_profiles/session.rs) | Freeze prompt, dictionary, short-term records and native destination authority for a recording.                                         | Freeze long-term text and revisions; stage correction batches.                                              |
| [request.rs](../../src-tauri/src/context_profiles/request.rs)                                                                   | Sends `text/operation/effect` JSON; short-term input contains text without record IDs.                                                  | Send identified records and validate text plus bounded memory changes.                                      |
| [feedback.rs](../../src-tauri/src/context_profiles/feedback.rs)                                                                 | `remember` accepts bounded text; legacy keyword effects check dictionary IDs; admission requires exact field readback.                  | Check source evidence, split admission rules, remove dictionary-dependent effects.                          |
| [actions.rs](../../src-tauri/src/actions.rs)                                                                                    | Runs live transcription, post-processing, History and queued paste; cleanup can notify the coordinator before queued output runs.       | Keep ownership until the output callback reports its outcome; admit staged memory at the appropriate point. |
| [llm_client.rs](../../src-tauri/src/llm_client.rs)                                                                              | OpenAI-compatible chat completion transport; no native tool request/response fields; History observers belong to transcription entries. | Add typed submission transport and bounded responses; give consolidation an independent operation owner.    |
| [ContextProfilesSettings.tsx](../../src/components/settings/context-profiles/ContextProfilesSettings.tsx)                       | Three tabs, dictionary rows, short-term remove/clear and serialized whole-profile autosave.                                             | Long-term text, instructions/action row, operation states and safe autosave boundaries.                     |

Current short-term limits are 20 records, 6,000 aggregate Unicode characters and 500 characters per record. Current `remember` validation proves neither that a statement was explicitly corrected nor that the model interpreted it correctly. Readback proves a field change. Windows capture currently reads native Edit controls; non-Windows context capture returns unavailable.

## Relationship to the pending processing-message work

[PM01 through PM06](../../plan.md#processing-message-implementation-task-graph) are pending. Their documents propose owned profile prompts, three readable request types, stronger range/output checks and message provenance. This memory delivery does not implement those unrelated changes or reopen their decisions.

Execute MR01 through MR09 before PM01. Reserve catalog version 3 for this delivery because the pending PM design already specifies version 2. The current supported migration is version 1 to version 3. A version-2 reader is needed only if an actual supported version-2 artifact exists when implementation starts; do not invent a decoder for unshipped schemas. If PM01 later changes owned prompt storage, allocate a later version and migrate the actual version-3 memory fields.

The memory requirements supersede dictionary-specific types, controls, effects and placeholders in the PM documents. PM implementations must consume long-term memory and the new proposal contract. The current inherited rewrite-prompt behavior and JSON data-envelope placement remain in force during this delivery. PM's later system/user message ownership remains governed by its own specification, substituting long-term memory for the removed dictionary. Reference memory must always be clearly delimited and cannot supply tool, profile or output authority.

## Data model and revision ownership

Working field names below are proposed implementation contracts. Rust symbol names may change; ownership and behavior must not.

Each profile stores `long_term_memory: String`, `long_term_memory_revision`, a rewrite-context revision, and an optional consolidation-instruction override. An absent override selects the built-in default displayed through i18next. Every profile owns its own memory and consolidation instructions, including profiles inheriting General's rewrite prompt.

Use distinct revision purposes:

- Catalog/metadata revision orders ordinary profile saves.
- Rewrite-context revision changes when routing, effective rewrite prompt or long-term input changes. Inherited General prompt changes must invalidate the effective prompt revision of affected sessions.
- Long-term revision changes only when its text changes through consolidation, Undo or deterministic migration.
- Short-term records have IDs and revisions; an invalidation epoch advances after explicit remove/clear or replacement operations. Ordinary append and automatic FIFO eviction do not advance it.

Renaming a profile, changing its icon or editing consolidation instructions must not invalidate a live correction. Saving an unchanged field does not advance the corresponding content revision. A long-term change can conservatively invalidate admission from an older rewrite snapshot.

Replace whole-profile write input with an edit DTO that omits long-term text and its revision. The backend owns revision values. Reject attempts to write forbidden fields instead of silently treating a stale full profile as a valid save. The UI merges authoritative save responses without overwriting a newer local instruction draft.

Short-term records retain text, stable identity, revision, source session, proposal index and locally assigned evidence category. Keep supporting correction evidence bounded. The provider receives only the IDs, revisions, text and minimal evidence needed for its task; native handles, cancellation generations and completion authority stay local.

## Migration and dictionary removal

Create an isolated migration module with explicit legacy catalog/profile/keyword types. Before the first upgrade, retain an exact local version-1 source backup. Preserve profile identity, order, names, icons, routing, prompt ownership and every canonical or misheard literal.

Serialize dictionary data deterministically into text, without a model call or semantic normalization:

```text
Terminology and transcription corrections:
- Preferred term: "BOM".
  Observed incorrect forms: "bomb".
- Preferred term: "Codex".
  Observed incorrect forms: "codecks", "code ex".
```

JSON-escape individual literals so quotes, line breaks, Unicode and case remain recoverable. Preserve canonical-only records. Empty dictionaries become empty long-term text. Preserve oversized valid legacy data rather than truncating it.

Convert exact parsed `{{dictionary}}` placeholders, including valid whitespace variants, into `{{long_term_memory}}` while preserving all other prompt prose. Keep original prompt text in the migration backup. Do not globally rewrite the word dictionary in user-authored content. Custom prose explicitly expecting keyword IDs or an array may require an actionable review warning; conversion cannot guarantee its semantics. Do not invent an automatic semantic prompt rewrite.

Migration must be repeatable without changes after the first successful save. Failed backup or persistence leaves the old store authoritative and publishes no new cache. Unknown versions and malformed data remain untouched with an explicit error. General-settings `custom_words` are ASR hints and are outside dictionary cleanup.

The final active runtime must contain no `Keyword`/`DictionaryEntry`, dictionary revision or collection fields, `add_misheard_form`, dictionary row editor/collision validation, or `{{dictionary}}` placeholder. Legacy names may remain only in migration code/tests, the exact backup and clearly historical evidence. Ordinary prose recommending a terminology dictionary inside long-term text is valid.

## Rewrite and short-term correction protocol

One response supplies final text, `insert` or `replace_selection`, and up to four proposed memory changes. An ordinary rewrite returns an empty change list. Each change supplies add/replace intent, text and bounded transcript evidence. Replacements must reference an existing ID and expected revision in the frozen short-term snapshot. Include optional terminology pairs for precise local checks; do not require every factual/contextual correction to fit a term pair.

The JSON response and native `submit_rewrite` function arguments normalize into the same internal type. Native mode requires exactly one known submission call, with parallel calls disabled when supported. Function arguments already contain the final text; Handy performs no additional model round trip to produce prose. Receiving a tool call never executes a memory write immediately.

Use a fixed schema with strict objects and no additional fields. Keep record IDs out of dynamically generated enums. Validate them locally. Tool-capability checks and JSON-schema capability are separate. The transport must handle a tools-only response with null textual content, refusals, truncation, malformed arguments and unexpected call counts. Rejected memory metadata cannot discard independently valid text; malformed overall result structure still blocks output safely.

Run a local validator before staging a batch:

1. Validate text operation against captured selection and current output rules.
2. Validate proposal count, record identity/revision, field sizes and Unicode-safe evidence boundaries.
3. Match quoted evidence to the actual microphone transcript. Captured reference text cannot authorize a memory request by itself.
4. For explicit terminology corrections, check observed/corrected terms and scope. Reject unsupported broadening and unknown replacement targets.
5. Deduplicate exact equivalent proposals; preserve case-sensitive corrected values. Semantic contradictions cannot be solved by lowercasing or hashing alone.

Model prompting must distinguish a correction from an ordinary rewrite, an example, a quotation, a joke, a negation or a changed mind about message content. An exact quote establishes source grounding, not semantic truth. Use evaluations to measure this remaining model-dependent judgment.

Initially support add/replace, with no automatic delete-all, arbitrary record deletion, full short-term replacement or long-term update tool. If a batch conflicts with a known record revision, reject the memory batch and preserve valid transcription output. Record a concise local reason; do not silently retain competing instructions as authoritative.

## Admission and output lifetime

Stage proposals on the recording session; assign evidence provenance locally.

| Proposal evidence                                                                         | Commit condition                                                                                                                                                                           |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Self-contained explicit user correction                                                   | Valid quoted transcript evidence; successful insertion dispatch and exact supported field/caret readback; current native target, session, cancellation, rewrite revision and memory epoch. |
| Correction depending on a selected span or model inference from an applied edit           | The same local gates plus exact supported field/caret readback of the expected output.                                                                                                     |
| Referential correction with unavailable referent, protected input or unsupported evidence | No memory admission.                                                                                                                                                                       |

Clipboard-only/no-paste output, blocked focus, failed processing/output, cancellation and stale sessions do not admit proposals in the first delivery. This policy does not expand automatic paste eligibility. If PM later withholds output for unknown selection, memory admission must respect that outcome.

Keep the coordinator and session alive until the queued output callback reports success, failure or blocking. A dispatch result reports Handy's configured output operation, not universal proof of acceptance by another application. Exact readback remains a separate bounded verifier. Do not hold the next recording indefinitely for verification; a newer session invalidates older pending admission under the existing authority rules.

Apply an admitted batch under one memory lock. Recheck cancellation, session, originating profile, rewrite revisions, record revisions and epoch immediately before commit. Use `(session_id, proposal_index)` and one consumed-batch identity instead of the current one-record-per-source-session deduplication. Enforce bounds and emit one update event. Duplicate responses, output callbacks and optional provenance upgrades cannot apply changes twice.

Classify evidence locally before staging. Under the retained policy, every batch waits for exact readback and a failed readback admits none. A later provenance upgrade changes metadata only and cannot reapply the batch.

Unsupported readback or missing native target identity prevents admission. Never label an explicit statement as verified insertion merely because its text was dispatched.

The earlier T07 decision records destination readback as the user's completion requirement. This plan preserves it. The analyses also proposed an optional dispatch-only path for self-contained explicit corrections, with native target/session checks and statement provenance. Adopting that path requires an explicit policy choice. If chosen later, mixed batches use the strongest gate: any edit-dependent proposal makes the entire batch wait for exact readback. Without that choice, implement the retained readback policy.

## Independent long-term consolidation

Create a dedicated operation owner, prompt builder and parser under the context-profile module. Proposed API responsibilities are start, query status, cancel and revision-safe Undo. Consolidation has its own operation ID, separate from live recording IDs, and at most one active run per profile. It must not acquire the recording coordinator or select a destination.

Before starting, flush valid pending instruction/profile edits. If saving fails or the profile has no persisted ID, retain the draft and send no request. Snapshot profile ID, existing long-term text/revision, short-term records and revisions/epoch, instruction text, provider/model/credentials, and the system-prompt version. Release locks before network work. Later provider or instruction changes affect the next run.

The dedicated system prompt instructs the model to produce durable profile knowledge, treat memory as reference data, preserve useful existing scope, honor the explicit consolidation instructions, and avoid inventing facts or silently resolving contradictions. It does not use the rewrite prompt or its `text/operation` contract.

Suggested default instruction text:

> Bring durable terminology, explicit transcription or translation corrections, and reusable context from short-term memory into long-term memory. Preserve useful existing guidance and its qualifications. Merge duplicates. Prefer explicit user corrections over inferred observations. Exclude temporary instructions, unsupported guesses and incidental personal details. Change existing guidance only when my instructions or a clear correction justify it.

The request contains only the required consolidation data:

```json
{
  "user_instructions": "Keep engineering terminology and translation preferences.",
  "existing_long_term_memory": "Use Codex when discussing the coding assistant.",
  "short_term_memory": [
    {
      "text": "Use BOM when bomb refers to this term.",
      "evidence": "explicit_user_statement"
    }
  ]
}
```

The response is a strict object containing one string:

```json
{
  "long_term_memory": "Use Codex when discussing the coding assistant. In engineering discussions, use BOM (bill of materials) when a transcription says bomb and context refers to this term."
}
```

Use schema enforcement where supported and the same local parser everywhere. Return unchanged text if nothing supported should be added. Reject malformed output, unexpected tools/fields, oversized content and an unexpected empty replacement of nonempty memory. Do not salvage arbitrary provider prose. A full replacement permits organization and deduplication; validation cannot prove that every previous fact survived. Keep one previous version and offer Undo.

On completion, merge only the originating profile's long-term field into the latest catalog under the profile-write lock. Require the captured long-term revision, a live uncancelled operation, a surviving profile and an unchanged source-invalidation epoch. Explicit remove/clear/replacement prevents stale promotion; a conservative first implementation can invalidate all outstanding snapshots for that profile on any such action. Newly appended notes do not invalidate the request and remain untouched. Automatic FIFO eviction caused by the existing bounds does not invalidate the frozen source snapshot: do not require every captured record to remain in the live list. Unrelated profile changes must survive the merge.

Persist before publishing the cache/event. Failed persistence restores the previous store/cache. Do not clear short-term memory or claim that every supplied note was incorporated. Undo requires the current long-term revision and restores only the previous long-term text; a later update cannot be overwritten by a stale Undo.

Typed operation states are running, committed, unchanged, cancelled, conflict and failed. Status remains discoverable across profile switching or component unmount. A restart retains the last committed long-term text and must not replay an unfinished request. Serialize cancellation and persistence/commit under the same operation/profile locking discipline, with a fixed lock order. Cancellation acknowledged before commit wins and prevents later persistence. If persistence already committed, a late Cancel returns the committed outcome rather than reporting cancellation. Cancellation cannot retract data already sent to the provider.

## Provider transport, limits and evidence

Reuse the configured HTTP provider/model with a frozen request configuration. Preserve the existing profile-path restriction on Apple Intelligence. No new model selector, model hosting, embeddings, MCP server or autonomous tool loop is needed.

Extend client options with a purpose/schema name and bounded response collection. The current client reads the entire HTTP body before parsing, so parser limits alone are insufficient. Keep tool result bodies inside existing configured request-history retention rules; tool arguments must not leak into ordinary diagnostics.

Proposed first-delivery limits:

| Data                                          | Limit                                                              |
| --------------------------------------------- | ------------------------------------------------------------------ |
| Short-term records and aggregate text         | Existing 20 records / 6,000 Unicode characters.                    |
| A short-term text / transcript evidence quote | 500 / 500 Unicode characters.                                      |
| Changes from one rewrite                      | 4.                                                                 |
| New long-term result                          | 32,000 Unicode characters.                                         |
| Consolidation instructions                    | 4,000 Unicode characters.                                          |
| Serialized consolidation input                | 256 KiB UTF-8.                                                     |
| Consolidation HTTP response                   | 512 KiB collected bytes, with the tighter parsed-text limit above. |
| Total consolidation deadline                  | 60 seconds.                                                        |

These are application bounds, not token counts. Provider-specific token parameters require confirmed support. Preserve larger migrated text, allow reading and unrelated metadata edits, and reject an oversized model request with a translated error. Never clip memory silently.

Cache capability results by endpoint/model/credential identity, transport mode and protocol version. Probes contain synthetic data only. Authentication failures, rate limits and server errors do not establish that tools are unsupported. Do not replay actual private context merely to discover another transport. A warm successful rewrite uses one provider request; consolidation uses one independent request and does not need the rewrite compatibility probe.

Consolidation must not attach itself to the latest transcription History entry. Existing `RunGuard` requires a real entry owner. Initially store minimal profile-operation metadata and the previous long-term text for Undo; use a separate local operation owner for diagnostics. Credentials, native handles and raw captured fields stay out of logs/events. Raw request archives remain governed by the existing archive preference; a generalized operation inspector is later work.

## Profile interface

Keep the existing three-tab navigation, renamed to Context selection, Long-term memory and Short-term memory. Long-term memory shows the current text, editable default/override instructions and the action button on its right. Fit the row within the existing 680 by 570 window; allow translated button text to wrap. The detail pane can scroll.

Show the captured provider/model for the operation, concise snapshot behavior, running status, Cancel, Updated/Unchanged, actionable errors and Undo. Do not block navigation with the component's global busy flag while a network request runs. Scope results by profile and operation ID so switching profiles cannot render another profile's result.

Short-term memory shows text and concise provenance, retaining remove/clear controls. Default instructions, labels, status and errors use i18next. New commands return typed error codes for translation. Disable consolidation when short-term memory is empty or the profile cannot be saved; profile setup itself remains local and does not require an endpoint/model.

## Verification and release criteria

Use synthetic text and local HTTP fixtures for automated provider tests. Native proof uses owned development settings and a supported foreground Edit fixture. Record actual coverage; do not equate a browser mock with real paste/readback or infer support for untested controls.

| Area                 | Required evidence                                                                                                                                                                                                           |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Migration            | Literal preservation including Unicode/newlines/quotes; empty and oversized legacy data; exact backup; repeat-load stability; unknown schema; failed save rollback; converted placeholders.                                 |
| Authority            | Autosave and rewrite/tool calls cannot change long-term text; instructions/name/icon edits do not discard a correction; actual context changes still invalidate stale admission.                                            |
| Short-term semantics | Ordinary rewrites do not learn; explicit corrections add/replace; unknown IDs, fabricated quotes, quotations/negation, contradiction and scope broadening reject memory safely.                                             |
| Lifecycle            | Multiple proposals commit once; cancelled/new sessions, focus changes, failed paste and clear-memory prevent stale effects; queued output completes before ownership release.                                               |
| Tools                | Null content with valid submission works; unexpected names/counts, truncation/refusal and malformed arguments fail safely; cache changes invalidate capability state; probes contain no private data.                       |
| Consolidation        | Distinct prompt/config snapshot; one active run per profile; unchanged output; malformed/empty/oversized results; deadline/cancel; delete/clear/replace sources; unrelated edits; persistence rollback; revision-safe Undo. |
| UI                   | Autosave failure retains drafts; switching/unmount does not overwrite results; keyboard access/status announcements; minimum width and long translations; flags and profile isolation.                                      |
| Cleanup              | Active types/API/UI/fixtures contain no dictionary model, legacy effect or placeholder. Remaining matches are migration, preserved user prose or explicitly historical evidence.                                            |

Add a versioned multilingual evaluation corpus for both correction proposals and consolidation. Include must-pass examples of quotations, negation, conflicting corrections, translation context and preservation of existing long-term facts. Compare JSON and tools under the same contract. Record false-learning count, correction recall, retention failures, request count and p50/p95 latency. A model changing transport does not establish a quality improvement. Any false learning in the must-pass examples blocks expanding the admission policy; no universal zero-error guarantee is implied.

Implementation verification commands run from the repository root: targeted `cargo test --manifest-path src-tauri/Cargo.toml --lib context_profiles`, relevant Playwright suites, `bun run build`, `bun run lint`, `bun run check:translations`, `bun run format:check` and `cargo clippy --manifest-path src-tauri/Cargo.toml`. Run full backend/browser suites at the integrated gate. Record baseline failures separately. These checks have not been run for this proposed implementation.

Release only after the whole graph passes, migration recovery is demonstrated, normal dictation/history/overlay behavior remains functional, and the dictionary cleanup audit is complete. Preserve earlier evidence and unresolved target limitations. Do not mark historical PM/T tasks done because this delivery was planned or verified.

Persistence implementation refinement: the profile catalog uses a dedicated atomic file owner, not plugin-store autosave. Writes sync an owned same-directory temporary file and replace the existing store before cache publication; the JSON root/key and migration backup format are retained. See [recovery evidence](../profile-memory-validation.md#native-proof-and-recovery).
