# Proposed reusable guidance: explain the purpose of a handoff

Use this guidance when a reader can see the steps in a workflow but must infer why work or information crosses a boundary. It applies to software, operations, and service workflows. The page should let a reader identify who owns each step, what is handed over, why the recipient needs it, and whether the shown behavior exists now or is proposed.

## Choose the representation

- Use responsibility lanes when ownership and parallel work matter. Put time on one axis, owners on the other, and draw a directional arrow for each material handoff. Mark the join where parallel results first depend on each other.
- Use a state diagram when allowed transitions matter more than owners. Use a comparison table when two versions or policies are the main question. Use a lifetime table when the reader asks what survives the next run. Do not add several views that repeat the same facts.
- At each important arrow, label the item or decision that moves and state the reason for crossing that boundary in plain language. Explain a safety or authority boundary beside the arrow that enforces it.
- Separate an item's authority, owner, and rate of change. Stable information may have limited authority; frequently changing information may still be an instruction. Do not use a single visual category to imply all three properties.
- Label current, proposed, and unverified claims in the main view. If a proposed change depends on a capability outside the workflow, state the dependency and avoid promising an outcome.

## Keep the reading path complete

Show the common path, parallel work, joins, and material failure cases without requiring a menu or stepper. Keep evidence and low-level details secondary, but cite the source of each current behavior. A diagram illustrates an interpretation; it does not prove that the full workflow ran. Give a text alternative that conveys the same sequence and reasons, and inspect desktop and narrow-screen rendering.

Use these comprehension prompts with a reader unfamiliar with the workflow when available; otherwise record that comprehension remains unverified: Who owns this step? What crosses the boundary? Why is that boundary here? Which parts are current, proposed, or unverified? Revise the labels if any answer requires guessing. Avoid arrows without direction, labels that merely rename the step, hidden exceptions, and claims of measured benefit without measurement.


## Terminology and visible product evidence

Check the project's glossary and established user-facing language before labeling a diagram. Explain necessary unfamiliar terms at first use; keep implementation names in supporting evidence when they do not help the main reading. State when a visual group represents several steps rather than a real component or feature. Retain useful whole-system diagrams and annotated captures of real interfaces, connecting them through consistent names or callouts. Preserve capture provenance and distinguish visible state from inferred processing. See the shared [portable instructions](../GENERIC-INSTRUCTIONS.md) for these principles.
