# Proposed portable explanation instructions

Review draft. Apply these principles according to the reader's task; do not require one diagram type or a fixed page skeleton. No installed skill has been changed.

## Establish the explanation's job

Identify the reader's question, the decision or understanding they need, and what they already know. Select only the relationships needed to answer that question. Establish current behavior from evidence. Separate documented rationale, an inferred explanation, and a proposed change; do not invent design intent to fill a diagram.

## Choose the representation

Use the project's language. Look for a glossary, domain model, UI labels, and terms already established with the reader. A term appearing in source code does not establish that the reader knows it. Prefer labels that name a concrete action or responsibility; introduce necessary technical terms with a brief explanation at first use and retain exact implementation names in supporting evidence. If no glossary exists, define only the terms this explanation needs.

Distinguish actual system entities from the diagram's groupings. If a box or lane groups several steps, say so; do not present an invented grouping as an application feature, component, or function. Explain the meaning of columns, containment, arrows, and ordering where it is not evident from the labels.

Preserve useful overview diagrams and annotated captures of real interfaces. Use the overview to explain relationships and the capture to locate the relevant behavior in the product. Connect them with consistent names or numbered callouts. Annotate actual controls without obscuring them, and retain capture provenance and limits. A screenshot establishes visible state, not hidden processing or later outcomes. Use a labeled mockup when a real capture is unavailable.

Use a flow when order and branching matter, responsibility lanes when ownership and handoffs matter, aligned examples when differences matter, and a state or lifetime view when transitions or persistence matter. Combine views only when each answers a distinct necessary question. Preserve names, directions and identifiers between views.

Keep independent dimensions distinct. Ownership, authority, change frequency, lifetime and implementation status do not necessarily divide a system at the same boundaries. Do not use one visual grouping to imply a relationship that the evidence does not support.

For consequential boundaries, communicate what crosses, who owns each side, why the separation matters, and what conditions the expected benefit depends on. Put a short explanation beside the boundary or in a clearly connected closeup. Do not repeat these questions as empty headings at every trivial step.

Explain a reason through its consequence or tradeoff. A “why” label that merely repeats the action adds no understanding. Prefer the problem avoided, the capability enabled, or the condition preserved, using language the intended reader can understand.

## Make relationships visible

Encode meaning with position, alignment, containment and connectors. Place concurrent work alongside each other and show where it meets. Attach an exception to its trigger or detecting stage and show the resulting consequence. Use comparable examples to expose what stays the same and what changes when that distinction explains the design.

Keep the main explanation and material alternatives readable without operating controls. Interaction may reveal optional evidence, highlight an existing relationship, or let the reader explore a meaningful variable. Do not make a menu, hover or repeated step navigation the only route to facts needed for comparison.

## Show status and uncertainty at the point of the claim

Distinguish observed behavior, behavior supported only by implementation evidence, proposed behavior, and unavailable capability. Place those distinctions on the affected node, connection or result. Use labels as well as visual styling. Describe benefits as conditional when they depend on configuration, external behavior or work that is not complete.

Separate a mechanism from its intended benefit and from proof that the benefit occurs. An accepted input, successful handoff or completed intermediate step does not automatically establish the final outcome. Show the missing condition where it matters.

## Keep the first reading focused

Lead with a compact whole-system view or a concrete example, according to the reader's question. Explain unfamiliar terms through their purpose before introducing protocol or implementation vocabulary. Keep traceable evidence near claims and put detailed excerpts after the main explanation. Avoid presenting the same account several times in slightly different containers.

Keep stable requirement and decision identifiers when they already exist. Separate recommendations from recorded decisions. Preserve meaningful tradeoffs and open questions without reopening settled choices.

## Verify the explanation

Inspect the rendered artifact at the intended viewing sizes and with the relevant input methods. Preserve causal relationships when content reflows. Verify links, legibility, contrast, overflow and optional interaction. Label illustrations, mockups and runtime captures according to their actual source.

Ask whether a reader can identify the owner of an important action, explain a consequential separation, trace an exception to its effect, and distinguish present behavior from an intended benefit. Select the questions relevant to the task rather than imposing this list on every explanation. Rendering checks support usability; user review establishes whether the explanation communicates successfully.

Revise the representation from observed misunderstanding. Keep examples specific, but write reusable instructions in terms of relationships, evidence and reader needs so they transfer across domains.
