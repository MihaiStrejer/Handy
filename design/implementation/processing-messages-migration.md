# Processing messages: profile and History migration

Status: Pending PM implementation. This design consumes the schema-3 [memory delivery](profile-memory.md); no schema-4 runtime is implemented yet. The schema-2 reservation in the original design was never shipped.

## Current integration points

The `profiles` key in `context-profiles.json` now stores schema 3. Each profile has a nullable inherited/owned prompt, free-text `long_term_memory`, backend-owned `long_term_memory_revision` and `rewrite_context_revision`, consolidation instructions, one Undo value and a last-operation receipt. Ordinary edits use `ProfileEdit`, not the complete stored `Profile`. The memory migration independently converts schema 1 directly to 3 with an exact backup.

The current four-variable validator renders references into a JSON user envelope. The prompt modal still offers General inheritance. PM01 changes these contracts together so the editor cannot reintroduce inheritance after migration. History remains at SQLite schema version 5, with exact provider-call archives governed by the existing controls.

## Proposed catalog schema 4

Retain routing IDs, icons, order, all memory text/revisions, consolidation instructions, Undo and receipts. Change only prompt ownership and readiness; keep memory writes outside `ProfileEdit`.

```json
{
  "schema_version": 4,
  "revision": 14,
  "next_id": 4,
  "profiles": [
    {
      "id": "profile-2",
      "name": "Mail",
      "revision": 14,
      "rewrite_context_revision": 14,
      "long_term_memory_revision": 9,
      "prompt_revision": 14,
      "icon": "mail",
      "rules": [{ "application": "mail.exe", "workspace": null }],
      "prompt": "Rewrite conversational dictation using {{long_term_memory}}.",
      "prompt_status": "ready",
      "long_term_memory": "Use scoped terminology and translation guidance.",
      "consolidation_instructions": null,
      "long_term_undo": null,
      "last_consolidation": null
    }
  ]
}
```

Every schema-4 profile owns its prompt. Server-owned `prompt_status` is `ready` or `needs_review`. `prompt_revision` advances only when prompt text changes or ownership is migrated. `rewrite_context_revision` retains its MR authority over routing/prompt changes; long-term revision changes only through explicit consolidation or Undo. Metadata/instruction saves cannot invalidate feedback merely by incrementing catalog revision.

Use one exact `DEFAULT_PROFILE_PROMPT_V4` seed for fresh General, new profiles and Use default. Its system framing is specified in [processing messages](../components/processing-messages.md#system-prompt), with one long-term placeholder. Copies are independently editable. Ordinary Post Process prompts remain untouched and do not seed schema 4.

Routing reads the selected profile's owned prompt and long-term text. General remains a routing fallback only. New session provenance uses `OwnedProfile`; old History source strings remain historical facts. A `needs_review` profile resolves its identity for History but cannot dispatch before the user converts its prompt.

### Prompt validation and saves

Allow a nonempty prompt of at most 32,000 Unicode scalars with zero or one `{{long_term_memory}}`. Reject dynamic transcript/input/short-term variables, repeated memory placeholders, malformed braces and unknown syntax. Render the frozen long-term string once; append a labeled section when the placeholder is absent. The fixed response contract remains outside editable prose.

Retain optimistic catalog revision checks and the narrow `ProfileEdit` command. Ignore attempts to choose backend status or revisions; reject memory/receipt fields as unknown DTO fields. A changed valid prompt becomes `ready` and advances prompt/context revisions. An unchanged `needs_review` prompt remains exactly stored and editable through unrelated valid metadata saves. Failed Save leaves the private modal draft open; Cancel, Escape and backdrop dismissal discard only that draft. Use default stages a copy and requires Save.

### Transactional schema-3 migration

Run under `PROFILE_WRITES` before publishing cache state. For each inherited profile, copy its exact effective schema-3 General prompt into an owned value. Preserve custom prose exactly. Only an exact match for the current shipped seed literal defined by `ProfileCatalog::seed` may become the new generic default automatically; do not infer seed identity from similar text or selected ordinary prompts.

For every other prompt, preserve the exact text. Dynamic variables, repeated memory placeholders, unsupported expressions or invalid bounds produce `needs_review`; compatible prose produces `ready`. The editor explains conversion and offers an explicit default replacement without stripping user instructions.

Set schema to 4 and advance catalog revision once using checked arithmetic. Advance each profile's metadata, prompt and rewrite-context revisions for the ownership change, retaining long-term revisions and all memory-owned fields byte-for-byte. Keep profile IDs and `next_id`. Loading schema 4 is idempotent. A failed conversion, backup or persist leaves the original schema-3 store/cache available. Preserve the MR schema-1 backup independently; use a separate exact schema-3 backup for this migration. Do not fabricate or accept a schema-2 transition.

Fresh installs seed schema 4 directly. Remove General prompt readiness as an enable-time gate; readiness belongs to the selected profile at request time. Record all new values with generated bindings and editor tests before advancing PM01.

## Additive History schema version 6

Append one migration to [`MIGRATIONS`](../../src-tauri/src/managers/history.rs). Do not rewrite version-5 rows or change existing provider-call/archive tables. Proposed SQL:

```sql
ALTER TABLE history_processing_runs ADD COLUMN request_type TEXT;
ALTER TABLE history_processing_runs ADD COLUMN context_mode TEXT;
ALTER TABLE history_processing_runs ADD COLUMN request_template_version INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN prompt_revision INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN long_term_memory_revision INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN request_archive_policy TEXT NOT NULL DEFAULT 'enabled';
```

The exact new run values are:

| Runtime classification                                       | `request_type`     | `context_mode`   | `request_template_version` |
| ------------------------------------------------------------ | ------------------ | ---------------- | -------------------------- |
| Verified empty field                                         | `new_message`      | `full`           | `1`                        |
| Verified nonempty field and cursor, complete field           | `continue_message` | `full`           | `1`                        |
| Verified cursor in bounded field prefix                      | `continue_message` | `excerpt`        | `1`                        |
| Verified selection, complete field                           | `edit_selection`   | `full`           | `1`                        |
| Verified selection with bounded field excerpt                | `edit_selection`   | `excerpt`        | `1`                        |
| Verified complete selection without usable surrounding field | `edit_selection`   | `selection_only` | `1`                        |
| No usable field/cursor and no verified selection             | SQL `NULL`         | `context_free`   | `1`                        |
| Legacy version-5 row                                         | SQL `NULL`         | SQL `NULL`       | SQL `NULL`                 |

`1` denotes the first text-template version, not the catalog schema or existing `metadata_version`. Keep `request_type` nullable so the context-free case is distinguishable by `context_mode` from legacy rows. Preserve preexisting `prompt_source` values such as `inherited` and `custom` on old rows. After resolution, new profile runs use `prompt_source = "owned_profile"` and store frozen `profile_revision`, `prompt_revision` and `long_term_memory_revision`. For ordinary post-processing and history retry, leave the new fields null unless those paths actually use this profile message contract.

Extend proposed `RunSnapshot`/`ProcessingRun` fields and `RunGuard::set_profile`. Add proposed `RunGuard::set_request_template_version(version)` and `RunGuard::set_request_classification(request_type, context_mode)` methods; each updates only a running row. Keep the existing `get_history_processing_runs`/`get_history_processing_run` command signatures; extend their returned structures and SQL mappings, then regenerate Specta bindings. [HistoryInspector.tsx](../../src/components/settings/history/HistoryInspector.tsx) should display a localized task and context state from the run even when there is no provider call or the request archive is off. Show an older row with all three null fields as legacy/unavailable, never as context-free. Do not make `RequestContents.prompt_template` authoritative; current `start_call` does not populate it. The run's archived `prompt_template` remains the selected profile's configured prose, while `request_json` remains the only exact sent transport body.

The order in [`request::process`](../../src-tauri/src/context_profiles/request.rs) is: resolve frozen context; call `set_profile` with owned provenance and revisions; call `set_request_template_version(1)` when this message path starts; check prompt readiness and protected capture; classify once; persist request type/context mode; assemble `PreparedRequest` and check the combined 512,000-byte system-plus-user content budget; perform the synthetic compatibility probe; send the rewrite; validate its operation against that same classification; then authorize output. The runtime author's `PreparedRequest` carries the classification and rendered strings on success; the classifier must make its result available for History before rendering or budget rejection. If assembly or budget validation fails after classification, the run keeps classification and has no provider call. If a selection is known but unverifiable, or prompt review blocks processing before classification, retain available template version/provenance and an error code, leave unsupported classification columns null, and create no provider call. A compatibility probe, when sent, remains a real `purpose = "compatibility"` call and carries no private prompt, long-term memory or capture.

Keep the existing archive controls: `save_history_request_contents = false` stores no body or prompt template; a body over the 1 MiB archive limit gets `omitted_too_large`; clearing archives removes `request_json`, `prompt_template` and the legacy `post_process_prompt` text. The new classification and revision integers are non-content metadata and remain after opt-out or clearing. Do not add captured field text, long-term text or private prompt text to the new columns. The exact system and user messages are already available within the retained serialized provider request body when archiving is enabled; do not add a second message archive.

### Initial provenance, failed writes and archive revocation

Change the profile-mode `RunSnapshot` construction in [`actions.rs`](../../src-tauri/src/actions.rs): initialize `prompt_template` and `prompt_source` as null until a frozen profile resolves, and set the intended `request_template_version` to 1 at run creation. Do not seed profile prompt content from the selected ordinary Post Process prompt, which the current code does before `request::process`. Preserve the ordinary path's selected-prompt snapshot. A profile failure before resolution therefore has unavailable prompt provenance rather than a falsely attributed ordinary prompt. Extend the replay rejection in [`commands/history.rs`](../../src-tauri/src/commands/history.rs) to recognize a non-null request-template version as well as legacy profile identity/source; a profile run that failed before resolution must not become eligible for ordinary History retry after its source is initialized as null.

All new provenance writes follow the existing best-effort metadata policy. Attempt them before the probe/rewrite, but a failed run creation or metadata update must trigger the visible details warning and incomplete state where possible, rather than discard a usable transcription or prediction. Continue only under the same in-memory task classification and output proof. Tests must distinguish successful durable provenance from database-failure cases; the UI cannot claim complete metadata in the latter.

The new `request_archive_policy` is persistent per-run authority with values `enabled`, `disabled` or `cleared`. Initialize new runs from the archive preference. The migration default affects only permission for future writes and does not populate legacy content. `set_profile` and `start_call` must read the row's policy and write content within the same transaction; remove reliance on `RunGuard.archive_enabled` as final authority. `clear_requests` sets every existing run's policy to `cleared` in the transaction that removes request and template contents. Later metadata writes for those runs are allowed, but later template/call content writes remain forbidden and new archive rows report `cleared` with no body. Runs created after the clear use the then-current preference.

Turning the archive preference off also revokes new content writes for already-running runs, using `disabled` unless already `cleared`, without deleting their previously retained contents. Turning it back on permits newly started runs; it does not restore permission on revoked runs. A disabled policy leaves already-retained templates intact until explicitly cleared, while a cleared policy always keeps content null. Serialize preference/run-start transitions under a shared archive-policy lock or equivalent generation gate so a run cannot start with stale enabled authority after opt-out completes. Clear and insert/update operations use database transactions so either the write precedes clear and is removed, or it follows clear and is refused. Retain metadata and existing deletion/no-resurrection behavior.

## Verification gates

- Migrate schema-3 fixtures for exact shipped fallback, inherited General, custom valid prose, each old dynamic variable, duplicate long-term memory placeholder and malformed expression; inspect serialized schema-4 values, IDs, long-term text, revisions and per-profile prompt independence after a General edit.
- Inject load, serialization and store-save failures; confirm the schema-3 value remains byte-for-byte available, the cache never exposes a partial schema-4 catalog, and a repeated successful load does not advance revisions.
- Save unrelated fields on a `needs_review` profile, reject profile dispatch for it before any probe, then convert in the modal; verify Save, Cancel, Escape, backdrop dismissal and Use default against persisted data and native focus behavior.
- Upgrade a real version-5 History fixture through the new SQLite migration; verify old rows retain null new fields and original prompt-source labels, while new full, excerpt, selection-only and context-free runs expose the tabled values before any provider call.
- Exercise malformed/oversized/protected requests and incompatible endpoints: assert no fabricated provider call, correct run metadata/error, exact archived messages only for actual calls, and opt-out/clear preservation of non-content metadata.
- Run targeted Rust storage/routing/history tests, generated-binding/front-end build and lint, then a native local-endpoint session with at least two isolated profiles; compare the archived transport body with the actual system/user messages and verify the History inspector's labels. Record any platform or provider behavior that could not be exercised.
