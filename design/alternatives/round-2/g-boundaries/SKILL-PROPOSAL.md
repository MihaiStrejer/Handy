# Proposed show-me guidance: explain a boundary when the reader asks why

This is a portable proposal for the show-me workflow. It is not an edit to the installed skill.

## Choose the visual from the reader's question

Before drawing, write the question the reader must answer and identify the evidence that can answer it. Use an ownership map when several actors pass work between them and the reader needs to understand responsibility. Use a state diagram when the important fact is which transitions are allowed. Use a timeline when timing or deadlines decide the result. Use an annotated capture when the question concerns an existing control or screen. Use a comparison table when the task is to choose among alternatives. A page may combine a small number of these forms when each answers a different question.

For an ownership map, draw the whole path first. Mark parallel work and joins explicitly; visual order must not imply a false causal order. Pick the few boundaries that carry the main explanation, then place their closeups near the map and keep them visible without interaction. Vary their shape to fit the question; do not turn every component into an identical card.

At each selected boundary, answer four questions in plain language: **What crosses? Why does this owner hand it off? What benefit or guarantee depends on the handoff? How does the next owner know whether it may continue?** Name the owner of each check. Separate a submitted action, an accepted result, and an observed outcome when those are different facts.

When describing a proposed optimization, draw the current flow and the proposed flow separately. State its required conditions, the measurement that would establish the benefit, and what work still happens for each operation. Do not infer a benefit from interface labels, component names, or stored state alone. Distinguish ownership, authority, and change frequency instead of treating them as one property.

Keep exceptions near the handoff they interrupt. For each, name the cause, the check, and the consequence. Mark absent implementation, unsupported surfaces, and unverified runtime behavior with different words from implemented behavior. Keep source detail available as secondary evidence without forcing the reader through it before the main explanation.

Do not use a boundary map for a single local action, a screen whose visible controls are the evidence, or a decision whose alternatives matter more than execution. In those cases choose the smaller capture, table, or sentence that answers the question.


## Terminology and visible product evidence

Check the project's glossary and established user-facing language before labeling a diagram. Explain necessary unfamiliar terms at first use; keep implementation names in supporting evidence when they do not help the main reading. State when a visual group represents several steps rather than a real component or feature. Retain useful whole-system diagrams and annotated captures of real interfaces, connecting them through consistent names or callouts. Preserve capture provenance and distinguish visible state from inferred processing. See the shared [portable instructions](../GENERIC-INSTRUCTIONS.md) for these principles.
