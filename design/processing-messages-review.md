# Processing messages: review and readiness

Scope: Review the [message specification](components/processing-messages.md), produce implementation documentation, then independently review it. The user requested GPT-6-Sol reviewers and parent adjudication. Implementation is not authorized in this work session.

Working directory: `D:/rust/Handy`. Preserve the running application, existing data and unrelated working-tree changes. No provider calls are needed for this documentation review.

## Review stages

| Stage | Status | Deliverable |
| --- | --- | --- |
| R1: Three independent specification reviews | done | Source-backed findings for semantics, capture/output, and migration/history. |
| R2: Parent adjudication and specification revision | done | Accepted/rejected findings and precise decisions below. |
| R3: Three implementation-document authors | done | [Runtime](implementation/processing-messages-runtime.md), [migration](implementation/processing-messages-migration.md) and [verification](implementation/processing-messages-verification.md). |
| R4: Implementation-document reviews | done | Two fresh independent reviewers plus one cross-review; reported blockers resolved and targeted rechecks completed. |
| R5: Parent readiness audit | done | Documents and dependencies validated; implementation ready to begin when authorized. |

The canonical implementation task graph remains root `plan.md`; the documentation will extend it with new pending tasks. Existing completed tasks remain unchanged.

## Adjudication

### Specification review

All three reviewers used GPT-6-Sol. The parent checked the reported seams in the current code and revised the specification before documentation drafting.

| ID | Finding and source | Parent decision |
| --- | --- | --- |
| S1 | Capture drops native selection offsets; repeated text cannot be located safely (`context_profiles/session.rs`, `capture.rs`). | Accepted. Add a verified UTF-16 range and extent; validate exact substrings and range boundaries. |
| S2 | Preflight compares text without range; dispatch checks focus after an earlier input check (`request.rs`, `actions.rs`). | Accepted. Add range-based comparison and a final bounded check after paste delay, adjacent to dispatch. Do not claim OS-level atomicity. |
| S3 | Unknown selection currently becomes insert and skips capture recheck (`request.rs`). | Accepted. Distinguish unknown from verified no selection. Context-free rewriting may produce a candidate, but unknown selection withholds automatic paste. Known nonempty unverified selection blocks before the provider. |
| S4 | `clipboard::paste` appends trailing space and can auto-submit. | Partly accepted. Bypass trailing-space modification for profile output. Retain the user's explicit auto-submit setting; changing submission policy is outside this message-format change. Verify it separately. |
| S5 | Current supported capture is bounded Windows native Edit, not browser/editor/terminal fields (`capture.rs`). | Accepted. Keep 250 ms, 4096-scalar field and 2048-scalar selection bounds. Full field when it fits, labeled prefix or verified selection-only fallback otherwise. No new control integration is implied. |
| S6 | Delimiter choices could alter a stable system message or collide with captured values. | Accepted. Deterministic collision checks; dictionary formatting depends only on prompt/dictionary. User-message markers depend on user-message values. No second template evaluation. |
| S7 | Nullable prompt inheritance and old dynamic placeholders need migration (`storage.rs`, `routing.rs`, `template.rs`). | Accepted. Materialize owned effective prompts, preserve custom prose, explicitly review incompatible templates, migrate only an exact known shipped seed automatically. New profiles use independent built-in defaults. |
| S8 | History lacks classification/template-version provenance (`managers/history.rs`, `history_processing.rs`). | Accepted. Add a new migration with nullable metadata; preserve legacy facts and archive controls. Use null request type plus context-free mode/current version for degraded requests. |
| S9 | Limits, protected-state rejection and classification authority are ambiguous. | Accepted. Reject any protected capture, use one classification, enforce the 512000-byte combined message-content limit before provider work and fail without silently pruning data. |

The conservative degraded-output rule and explicit prompt conversion are recorded as implementation decisions. They affect existing unsupported controls and legacy custom prompts and must appear in the implementation documentation and release verification. No unresolved product question remains from this review round.

### Implementation-document review setup

The orchestration thread limit rejected a third fresh reviewer and restoration of an older idle reviewer. Two fresh GPT-6-Sol agents review runtime and migration independently. The migration author performs a bounded cross-review of the verification document and parent-authored PM01–PM06 graph, which that agent did not write; it does not review its own migration document. The parent reviews the combined result. This distinction is retained so the record does not overstate reviewer independence.

### Implementation-document findings and resolutions

| ID | Finding | Parent resolution |
| --- | --- | --- |
| I1 | Reliable-paste cancellation can preserve the candidate under CopyToClipboard, and guard errors can fall back to legacy paste. | Accepted. Require a typed non-fallback blocked result, a dedicated owned-clipboard abort overriding preservation, worker cleanup acknowledgment and no chord/submit. |
| I2 | Reliable paste does not currently receive the pre-paste delay; adding a sleep after publication can consume its timeout. | Accepted. Pay the profile delay once before publication; final guard follows publication immediately before the chord. Any allowed legacy fallback retains the paid-delay state and performs a fresh check. |
| I3 | Documents promise durable provenance before calls while current History writes are best effort. | Accepted clarification; rejected mandatory dispatch failure on metadata errors. Preserve useful processing with visible incomplete-details warnings and in-memory output proof. Attempt metadata first, but do not claim it was stored when storage failed. |
| I4 | Profile run creation initially archives the ordinary prompt before profile resolution. | Accepted. Start profile prompt/source null, retain intended template version and extend retry rejection so unresolved profile runs cannot become ordinary replay. |
| I5 | Clear archives can be undone by `set_profile` or a later call on the same run. | Accepted. Add durable per-run archive authority in migration 6; transactional clear revokes existing runs. Opt-out serializes with run creation/content writes; re-enabling permits only new runs. Add paused-run race fixtures. |
| I6 | PM01 promises a readiness gate without owning request dispatch. | Accepted. Add `request.rs` ownership and preserve typed readiness through resolution, with a specific zero-call request error. |
| I7 | Offscreen adapter fixtures and History retry do not prove a live foreground input-to-paste pipeline. | Accepted. PM06 owns a foreground Edit helper, an explicitly gated development-only synthetic-audio entry into the real session pipeline, and the orchestrator. Native output remains incomplete if focused HWND cannot be verified. |
| I8 | Output verification omits reliable fallback/abort and unsupported ExternalScript behavior. | Accepted. Add guarded method matrix, exact timing, ownership-sensitive cleanup and typed blocked-history assertions. |
| I9 | PM03 HTTP checks precede History schema integration. | Accepted clarification. PM03 proves assembly/HTTP in fixtures; PM05 retests actual run dispatch with migration/provenance before release. No intermediate build is a complete delivery. |
| I10 | Public prompt selection cannot restore null during native cleanup. | Accepted. Document the existing loaded-store fallback and verify restored settings after restart; a disposable portable fixture remains an alternative. |
| I11 | Earlier root-plan D7 still describes inherited prompts. | Accepted. Add a top-level supersession pointer while preserving historical completed task evidence. |

The parent made these documentation changes and requested targeted rechecks of the reported blockers. Both fresh reviewers confirmed closure of the runtime/provenance/archive contracts. The cross-review confirmed task ownership, dependency gates, cleanup and output-test coverage. A final wording conflict still described the native output fixture as offscreen; the parent corrected it to require the focused foreground helper and verified startup fixture flag. Offscreen fixtures remain adapter-only evidence. No application code was changed to resolve these findings in this planning session.

## Readiness

Ready to implement PM01–PM06. There is no unresolved product decision or documentation blocker in this scope. The [implementation index](implementation/processing-messages.md) links the three design documents and the canonical task graph. All six implementation tasks remain pending.

Validation: checked local Markdown targets, balanced code fences, trailing whitespace and unique task IDs/dependencies; PM01 → PM02 → PM03 → PM04 → PM05 → PM06 is acyclic. A 301-file source/test/script/config baseline remained unchanged, with no new source, test or script files. No application tests, native proof, provider calls or app restart were performed for this documentation-only work.

Implementation risks remain explicit acceptance work: the foreground native fixture and development entry point must be built, the final guard cannot lock external applications atomically, unsupported controls produce candidates without automatic paste when selection is unknown, and legacy custom prompts may need conversion. These are specified behaviors and future verification obligations, not claims that the running app already supports them.
