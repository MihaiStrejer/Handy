# Processing messages: implementation documentation

Status: Reviewed and ready for implementation. Implementation has not started. The [specification](../components/processing-messages.md) is the product contract; the [review record](../processing-messages-review.md) explains the parent decisions, independent reviews, cross-review and readiness assessment.

Memory integration update (2026-10-02): Complete the [profile-memory delivery](profile-memory.md) before this graph. Its long-term text and correction proposals replace long-term memory-specific portions of these documents. Catalog version 4 is reserved for this pending design, migrating the actual memory catalog version 3. Prompt ownership, request classification, readable messages and output guards remain governed by this design.

## Read in this order

1. [Runtime design](processing-messages-runtime.md): capture evidence, request classification, deterministic messages, response validation and guarded output.
2. [Migration and integration](processing-messages-migration.md): owned profile prompts, editor behavior, catalog upgrade, History schema and provenance.
3. [Verification plan](processing-messages-verification.md): test cases, commands, local endpoint fixtures and native evidence.
4. [Canonical task graph](../../plan.md#processing-message-implementation-task-graph): PM01–PM06, dependencies, file ownership and acceptance checks.

The three documents describe one implementation. Proposed symbols are design contracts until created in code; existing source links identify current integration points. No implementation task is complete merely because its design was reviewed.

## Delivery sequence

| Task | Outcome                                                       | Dependency |
| ---- | ------------------------------------------------------------- | ---------- |
| PM01 | Owned prompt configuration, migration and review UI           | MR09       |
| PM02 | Verified input ranges and capture completeness                | PM01       |
| PM03 | Three contextual message templates and one classification     | PM02       |
| PM04 | Exact profile fragments with final output verification        | PM03       |
| PM05 | Immutable message provenance and request archives in History  | PM04       |
| PM06 | Verified complete native pipeline with restored test settings | PM05       |

The sequence is intentionally ordered because the tasks share session types, bindings and output paths. Release only after the integrated verification gate passes.

## Behavioral changes to make visible

- Existing custom prompts using transcript, context or memory variables need explicit conversion in the prompt modal; stored text is preserved. Exact known shipped seed prompts migrate automatically.
- Each profile owns its prompt and long-term memory. Changing General cannot change another profile's instructions.
- Unsupported controls can yield a rewritten candidate, but unknown selection state withholds automatic paste. History retains the candidate for copying.
- Profile output preserves the model's fragment exactly, bypassing automatic trailing-space addition. The user's explicit auto-submit preference remains in force after successful dispatch.
- Windows native Edit remains the verified capture scope. Full fields are included within the retained bounds; excerpts and unavailable context are labeled truthfully.
