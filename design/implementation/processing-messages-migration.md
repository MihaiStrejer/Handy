# Processing messages: profile and History migration

Status: Implementation plan for the agreed [message specification](../components/processing-messages.md) and [review decisions](../processing-messages-review.md). Names introduced below are proposed implementation symbols; serialized values and migration behavior are the contract for this delivery. This document does not describe behavior already present in the running app.

## Current integration points

The profile catalog lives under the `profiles` key in `context-profiles.json`. Catalog v1 has one required General profile, an optional `prompt` on each other profile, and a catalog revision used for optimistic saves. A null prompt inherits General at resolution time; `get_context_profiles`, `save_context_profile` and `delete_context_profile` load the same catalog through the store lock. See [storage.rs](../../src-tauri/src/context_profiles/storage.rs), [routing.rs](../../src-tauri/src/context_profiles/routing.rs) and [session.rs](../../src-tauri/src/context_profiles/session.rs).

The current [validator](../../src-tauri/src/context_profiles/template.rs) accepts four prompt variables and the [request assembler](../../src-tauri/src/context_profiles/request.rs) refers to them as fields in a JSON user envelope. The [prompt modal](../../src/components/settings/context-profiles/ContextProfilesSettings.tsx) creates new profiles with `prompt: null`, shows inherited text, and offers reset to General. These contracts change together; a backend-only migration would leave the editor able to reintroduce inheritance.

History is currently at SQLite schema version 5. [history.rs](../../src-tauri/src/managers/history.rs) defines `history_processing_runs`, `history_provider_calls` and `history_request_contents`. [history_processing.rs](../../src-tauri/src/managers/history_processing.rs) starts the run before provider work, stores the profile template when archiving is enabled, and archives the exact serialized transport body per actual call. The new columns belong on the run so a rejected request can retain its classification without creating a fictitious provider call.

## Catalog v2 shape

Keep `ProfileCatalog`, `Profile`, `Keyword` and routing identities; change the serialized prompt fields as follows. The new `prompt_status` is server-owned, and the frontend must display it but must not choose its value on save.

```json
{
  "schema_version": 2,
  "revision": 14,
  "next_id": 4,
  "profiles": [{
    "id": "profile-2",
    "name": "Mail",
    "revision": 14,
    "dictionary_revision": 9,
    "prompt_revision": 14,
    "icon": "mail",
    "rules": [{"application": "mail.exe", "workspace": null}],
    "prompt": "You are a conversational rewrite engine. ... {{dictionary}}",
    "prompt_status": "ready",
    "dictionary": []
  }]
}
```

`prompt` is a required owned string on every v2 profile, including General. `prompt_status` is either `ready` or `needs_review`. `prompt_revision` is a catalog-revision value advanced only when the owned prompt text changes or is converted from v1; `dictionary_revision` advances only when dictionary content changes. `revision` still advances for every saved profile change. Existing IDs, `next_id`, rules, icons, dictionaries, memory ownership and the catalog's profile order remain unchanged. The example's ellipsis is illustrative, not stored text. On v2 load, structurally validate the catalog and apply the v2 prompt validator to `ready` prompts only; `needs_review` must preserve its invalid legacy text and remain editable after restart.

Use one exact built-in `DEFAULT_PROFILE_PROMPT_V2` for fresh General profiles, newly created profiles, and the modal's Use default action. Its proposed text is the system-prompt example in the [specification](../components/processing-messages.md#system-prompt), including one `{{dictionary}}` inside `<profile_dictionary>`. Treat it as copied seed content, not a shared mutable parent. Keep `post_process_prompts` and the selected ordinary Post Process prompt in [settings.rs](../../src-tauri/src/settings.rs) untouched; v2 seeding must not read them.

After migration, `routing::resolve` reads only the selected profile's prompt and dictionary. It no longer looks up General as a prompt owner. Remove `PromptSource::Inherited` from new session snapshots; use proposed `PromptSource::OwnedProfile` for a resolved v2 profile, including General. Preserve old History source strings as historical data. The frozen context carries `prompt_status`, `prompt_revision` and `dictionary_revision` from that selected profile. General remains the routing fallback only. Routing must not reject a `needs_review` prompt before the run can record the selected profile; `request::process` rejects it before assembly or provider work.

### Prompt validation and save contract

The v2 prompt validator accepts a nonempty prompt of at most 32,000 Unicode scalars with zero or one `{{dictionary}}` placeholder. It rejects `{{transcript}}`, `{{input_context}}`, `{{short_term_memory}}`, repeated dictionary placeholders, malformed braces and unknown expressions. The renderer substitutes the selected dictionary in one pass; if the placeholder is absent, it appends one labeled dictionary section. It never reinterprets dictionary values as template syntax. The fixed response contract remains outside the editable prompt.

Retain the existing `save_context_profile(profile, expected_revision)` command signature and optimistic catalog revision check. Change its backend behavior according to the persisted profile, not the client's `prompt_status` field:

| Save case | Backend result |
| --- | --- |
| New profile | Require a valid owned prompt. Ignore any client-supplied status, assign `ready`, a stable ID and new prompt revision. |
| Existing `ready` profile, prompt changed | Validate the new prompt; save it with `ready` and a new prompt revision. |
| Existing `needs_review` profile, prompt changed | Validate the new prompt; only a valid changed value converts it to `ready`. |
| Existing `needs_review` profile, prompt unchanged | Permit valid unrelated edits, keep the stored prompt and `needs_review` status exactly. |
| Existing profile, prompt unchanged | Do not change prompt revision. Advance dictionary revision only if dictionary content changed. |

Do not let a client set `ready` or choose prompt, dictionary or profile revisions by editing serialized fields. Use `expected_revision` for the stale-write check, look up the stored profile by ID, compare the incoming prompt and dictionary with stored content, then derive status and all revision values on the server. Return the authoritative catalog from each successful save as today. The prompt modal's Save uses the same command with its private prompt draft; ordinary debounced profile edits use the current stored prompt and can proceed while it needs review. A failed prompt Save keeps the modal open with the validation error. Cancel, Escape and backdrop dismissal discard only the private draft and leave catalog data unchanged. The Use default action copies `DEFAULT_PROFILE_PROMPT_V2` into that draft and commits only on Save. Remove Use General prompt, Customize, inherited previews and the other three variable buttons. The editor shows only `{{dictionary}}`, the effective owned text, and a localized review explanation listing incompatible variables. A `needs_review` profile remains selectable/editable but profile-mode dispatch fails before compatibility probing with a recoverable prompt-review error.

Update [English strings](../../src/i18n/locales/en/translation.json) and corresponding locale keys for owned prompt, Use default, review state, and the single variable. Generate [bindings.ts](../../src/bindings.ts) from the changed Rust types so `Profile.prompt` is `string` and `prompt_status`/`prompt_revision` are visible. Keep the ordinary Post Process prompt editor and its `${output}` token unchanged.

## Catalog v1 to v2 migration

Run migration under `PROFILE_WRITES` before any catalog is published to `ProfileCache`. Read the original store value, deserialize and validate the v1 *structure*—required fields, unique General, usable effective prompt source for each profile and catalog identity/revision invariants—without applying the new template validator to old prompt text. A structurally invalid catalog fails untouched; an old prompt with incompatible syntax is migrated as `needs_review`. Build a complete v2 value in memory and persist it once. Reject unknown versions without seeding an empty replacement. After a successful store save, publish the complete v2 catalog to the cache. If saving fails, restore the original value in the live store exactly as [`persist`](../../src-tauri/src/context_profiles/storage.rs) does today, leave the cache unpublished or at its previous good value, and return an error. Do not start profile requests against a partially converted catalog.

Use the exact current shipped v1 fallback literal as the sole automatic seed match:

```text
Rewrite {{transcript}} accurately using {{dictionary}} and {{short_term_memory}}. Use {{input_context}} when available.
```

The literal is defined in [`ProfileCatalog::seed`](../../src-tauri/src/context_profiles/storage.rs). Only a v1 prompt byte-for-byte equal to it becomes `DEFAULT_PROFILE_PROMPT_V2` without review. This check applies to each profile's resolved effective v1 prompt, so an inherited copy of that exact fallback is converted independently. Do not infer a shipped seed from the selected ordinary prompt, from prompt names, or from similar wording.

For every other profile, first resolve its effective v1 prompt against the v1 General prompt and copy that exact text into its own v2 `prompt`. This includes inherited profiles; later General edits cannot alter their copies. If the copied prompt contains any old dynamic placeholder, a repeated dictionary placeholder, malformed/unsupported expression, or violates the v2 prompt bounds, retain its exact text with `prompt_status: "needs_review"`. A prompt with no incompatible syntax remains `ready` even if its wording came from a formerly selected ordinary prompt. Do not delete an invalid legacy string to satisfy the new validator. In the modal, identify the invalid tokens and offer an explicit default replacement; do not strip them on load or Save. A custom prompt and an inherited prompt with equal text each become independent owned values.

Set `schema_version` to 2 and advance catalog revision once with checked arithmetic. Set `revision` and `prompt_revision` of every migrated profile to the new catalog revision because ownership and validation semantics changed; retain each existing `dictionary_revision`, since dictionary content is unchanged. Preserve `next_id` and each profile ID. The migration is idempotent: loading v2 performs no conversion and no revision increment. A failed conversion or revision overflow returns an error and leaves v1 on disk. Verify that an older build can still parse the untouched v1 store after an injected persistence failure.

The fresh-install path creates v2 General directly with the built-in prompt, `ready` status and zero revisions. A new profile in the UI starts with its own copy of the same default before its first save. No extra migration command is needed; `get_context_profiles` and existing save/delete commands trigger the locked load path. Remove the current General-template validation from `change_post_process_profiles_setting` in [storage.rs](../../src-tauri/src/context_profiles/storage.rs); the selected profile's readiness is checked at request time, so one legacy General prompt does not block use of an independently ready profile.

## Additive History schema version 6

Append one migration to [`MIGRATIONS`](../../src-tauri/src/managers/history.rs). Do not rewrite version-5 rows or change existing provider-call/archive tables. Proposed SQL:

```sql
ALTER TABLE history_processing_runs ADD COLUMN request_type TEXT;
ALTER TABLE history_processing_runs ADD COLUMN context_mode TEXT;
ALTER TABLE history_processing_runs ADD COLUMN request_template_version INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN prompt_revision INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN dictionary_revision INTEGER;
ALTER TABLE history_processing_runs ADD COLUMN request_archive_policy TEXT NOT NULL DEFAULT 'enabled';
```

The exact new run values are:

| Runtime classification | `request_type` | `context_mode` | `request_template_version` |
| --- | --- | --- | --- |
| Verified empty field | `new_message` | `full` | `1` |
| Verified nonempty field and cursor, complete field | `continue_message` | `full` | `1` |
| Verified cursor in bounded field prefix | `continue_message` | `excerpt` | `1` |
| Verified selection, complete field | `edit_selection` | `full` | `1` |
| Verified selection with bounded field excerpt | `edit_selection` | `excerpt` | `1` |
| Verified complete selection without usable surrounding field | `edit_selection` | `selection_only` | `1` |
| No usable field/cursor and no verified selection | SQL `NULL` | `context_free` | `1` |
| Legacy version-5 row | SQL `NULL` | SQL `NULL` | SQL `NULL` |

`1` denotes the first text-template version, not the catalog schema or existing `metadata_version`. Keep `request_type` nullable so the context-free case is distinguishable by `context_mode` from legacy rows. Preserve preexisting `prompt_source` values such as `inherited` and `custom` on old rows. After resolution, new profile runs use `prompt_source = "owned_profile"` and store frozen `profile_revision`, `prompt_revision` and `dictionary_revision`. For ordinary post-processing and history retry, leave the new fields null unless those paths actually use this profile message contract.

Extend proposed `RunSnapshot`/`ProcessingRun` fields and `RunGuard::set_profile`. Add proposed `RunGuard::set_request_template_version(version)` and `RunGuard::set_request_classification(request_type, context_mode)` methods; each updates only a running row. Keep the existing `get_history_processing_runs`/`get_history_processing_run` command signatures; extend their returned structures and SQL mappings, then regenerate Specta bindings. [HistoryInspector.tsx](../../src/components/settings/history/HistoryInspector.tsx) should display a localized task and context state from the run even when there is no provider call or the request archive is off. Show an older row with all three null fields as legacy/unavailable, never as context-free. Do not make `RequestContents.prompt_template` authoritative; current `start_call` does not populate it. The run's archived `prompt_template` remains the selected profile's configured prose, while `request_json` remains the only exact sent transport body.

The order in [`request::process`](../../src-tauri/src/context_profiles/request.rs) is: resolve frozen context; call `set_profile` with owned provenance and revisions; call `set_request_template_version(1)` when this message path starts; check prompt readiness and protected capture; classify once; persist request type/context mode; assemble `PreparedRequest` and check the combined 512,000-byte system-plus-user content budget; perform the synthetic compatibility probe; send the rewrite; validate its operation against that same classification; then authorize output. The runtime author's `PreparedRequest` carries the classification and rendered strings on success; the classifier must make its result available for History before rendering or budget rejection. If assembly or budget validation fails after classification, the run keeps classification and has no provider call. If a selection is known but unverifiable, or prompt review blocks processing before classification, retain available template version/provenance and an error code, leave unsupported classification columns null, and create no provider call. A compatibility probe, when sent, remains a real `purpose = "compatibility"` call and carries no private prompt, dictionary or capture.

Keep the existing archive controls: `save_history_request_contents = false` stores no body or prompt template; a body over the 1 MiB archive limit gets `omitted_too_large`; clearing archives removes `request_json`, `prompt_template` and the legacy `post_process_prompt` text. The new classification and revision integers are non-content metadata and remain after opt-out or clearing. Do not add captured field text, dictionary entries or private prompt text to the new columns. The exact system and user messages are already available within the retained serialized provider request body when archiving is enabled; do not add a second message archive.

### Initial provenance, failed writes and archive revocation

Change the profile-mode `RunSnapshot` construction in [`actions.rs`](../../src-tauri/src/actions.rs): initialize `prompt_template` and `prompt_source` as null until a frozen profile resolves, and set the intended `request_template_version` to 1 at run creation. Do not seed profile prompt content from the selected ordinary Post Process prompt, which the current code does before `request::process`. Preserve the ordinary path's selected-prompt snapshot. A profile failure before resolution therefore has unavailable prompt provenance rather than a falsely attributed ordinary prompt. Extend the replay rejection in [`commands/history.rs`](../../src-tauri/src/commands/history.rs) to recognize a non-null request-template version as well as legacy profile identity/source; a profile run that failed before resolution must not become eligible for ordinary History retry after its source is initialized as null.

All new provenance writes follow the existing best-effort metadata policy. Attempt them before the probe/rewrite, but a failed run creation or metadata update must trigger the visible details warning and incomplete state where possible, rather than discard a usable transcription or prediction. Continue only under the same in-memory task classification and output proof. Tests must distinguish successful durable provenance from database-failure cases; the UI cannot claim complete metadata in the latter.

The new `request_archive_policy` is persistent per-run authority with values `enabled`, `disabled` or `cleared`. Initialize new runs from the archive preference. The migration default affects only permission for future writes and does not populate legacy content. `set_profile` and `start_call` must read the row's policy and write content within the same transaction; remove reliance on `RunGuard.archive_enabled` as final authority. `clear_requests` sets every existing run's policy to `cleared` in the transaction that removes request and template contents. Later metadata writes for those runs are allowed, but later template/call content writes remain forbidden and new archive rows report `cleared` with no body. Runs created after the clear use the then-current preference.

Turning the archive preference off also revokes new content writes for already-running runs, using `disabled` unless already `cleared`, without deleting their previously retained contents. Turning it back on permits newly started runs; it does not restore permission on revoked runs. A disabled policy leaves already-retained templates intact until explicitly cleared, while a cleared policy always keeps content null. Serialize preference/run-start transitions under a shared archive-policy lock or equivalent generation gate so a run cannot start with stale enabled authority after opt-out completes. Clear and insert/update operations use database transactions so either the write precedes clear and is removed, or it follows clear and is refused. Retain metadata and existing deletion/no-resurrection behavior.

## Verification gates

- Migrate v1 fixtures for exact shipped fallback, inherited General, custom valid prose, each old dynamic variable, duplicate dictionary placeholder and malformed expression; inspect serialized v2 values, IDs, dictionaries, revisions and per-profile prompt independence after a General edit.
- Inject load, serialization and store-save failures; confirm the v1 value remains byte-for-byte available, the cache never exposes a partial v2 catalog, and a repeated successful load does not advance revisions.
- Save unrelated fields on a `needs_review` profile, reject profile dispatch for it before any probe, then convert in the modal; verify Save, Cancel, Escape, backdrop dismissal and Use default against persisted data and native focus behavior.
- Upgrade a real version-5 History fixture through the new SQLite migration; verify old rows retain null new fields and original prompt-source labels, while new full, excerpt, selection-only and context-free runs expose the tabled values before any provider call.
- Exercise malformed/oversized/protected requests and incompatible endpoints: assert no fabricated provider call, correct run metadata/error, exact archived messages only for actual calls, and opt-out/clear preservation of non-content metadata.
- Run targeted Rust storage/routing/history tests, generated-binding/front-end build and lint, then a native local-endpoint session with at least two isolated profiles; compare the archived transport body with the actual system/user messages and verify the History inspector's labels. Record any platform or provider behavior that could not be exercised.
