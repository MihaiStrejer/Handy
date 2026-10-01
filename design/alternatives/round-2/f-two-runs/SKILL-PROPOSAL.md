# Proposed generic addition to show-me

When a request repeats an operation, consider an aligned comparison of two representative runs. Use the same owner or stage rows for both runs so the reader can see what stays the same, what changes, and which handoffs repeat. Choose concrete examples from verified evidence where available; label invented examples as synthetic.

Separate current behavior from a proposed reuse or optimization. A value's position, interface label, or storage location does not by itself establish its lifetime or whether repeated work is avoided. Mark its actual lifetime and owner, identify what invalidates it, and show where the proposed boundary would move. Keep any change in trust or authority explicit.

For performance claims, show the measurement needed to establish the effect and identify conditions that may prevent reuse. Repeated-looking inputs alone do not establish avoided work or savings. Keep failures and open decisions discoverable in the main page, with stable option IDs where a decision is needed. Put detailed source excerpts after the main comparison.

Use this form only when the second run reveals a meaningful difference or reuse question. For a single-use flow, use a sequence, state diagram, or another form that answers the reader's question more directly.


## Terminology and visible product evidence

Check the project's glossary and established user-facing language before labeling a diagram. Explain necessary unfamiliar terms at first use; keep implementation names in supporting evidence when they do not help the main reading. State when a visual group represents several steps rather than a real component or feature. Retain useful whole-system diagrams and annotated captures of real interfaces, connecting them through consistent names or callouts. Preserve capture provenance and distinguish visible state from inferred processing. See the shared [portable instructions](../GENERIC-INSTRUCTIONS.md) for these principles.
