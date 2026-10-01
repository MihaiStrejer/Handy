# Proposed changes to show-me

Status: review draft. These instructions have not been installed. The alternatives change how Handy's architecture is explained; they do not change its architecture or settle the learning policy.

## Start with the reader's question

Before choosing a page structure, write the one question the reader should be able to answer after looking at the main visual. For this request: “What happens from starting Handy to producing text, and where can that path stop?” Identify the relationships needed to answer it: order, concurrency, ownership, data transformation, scope, and persistence.

Choose a representation for those relationships. A fixed sequence of whole, pieces, reference, gaps and decisions can remain an evidence checklist without requiring the same visible sections on every page.

| Reader's question | Useful representation | What it should make visible |
| --- | --- | --- |
| What happens, and where can it stop? | A: causal flow map | Main path, split/join, gates, failure branches |
| What happens to my words? | B: session storyboard | One concrete input, its transformations, intended output |
| Who does this, and what carries forward? | C: responsibility lanes and lifetimes | Owners, handoffs, concurrency, storage boundaries |
| What do I need to understand first? | D: concise overview | Whole path, request boundary, comparable outcomes |

Use more than one representation only when each answers a different necessary question. Avoid repeating the same story as cards, a stepper, a table and several paragraphs.

## Make the first reading complete

Show the primary path, material exceptions and unfinished boundary without requiring a dropdown, a tab change, repeated Next clicks or hover. Scrolling is allowed. Keep source excerpts collapsible when they are supporting evidence rather than the explanation itself.

Use interaction when changing something teaches a relationship: highlight the route taken, compare a changed assumption, or reveal a value along a continuous scale. Keep the full set of relevant cases visible when their comparison is the lesson. A dropdown remains appropriate for choosing a value from a large set; it should not serve as the table of contents for five essential facts.

## Give placement a meaning

- Put a failure beside the stage that detects it. Show cause, stopped action and resulting experience.
- Draw concurrent work beside each other and mark where the results meet. Step numbers must not imply that independent work waits.
- Put the states being compared in the same coordinate system. Keep labels and stage order consistent across examples.
- Use arrows for actual flow or handoff, containment for ownership, and aligned columns for comparison. Do not draw decorative connectors between unrelated facts.
- Mark an absent capability at the point where the flow needs it. Do not draw a completed feedback loop and explain its absence only in a footer.

## Keep examples and evidence precise

Carry one small, synthetic example through the explanation when it reduces abstract terminology. Show exact field names at boundaries where they matter, and mark omitted contract text. Name a change of scenario explicitly: a native Edit selection example must not imply that Windows Terminal selection is supported.

Place a short source reference near the claim. Put detailed code, hashes and test records after the main visual. Distinguish code-derived behavior, observed runtime behavior and proposals. A browser screenshot of an explainer proves its rendering, not the app behavior it depicts.

Keep stable requirement and decision IDs. A choice of presentation A–D is separate from a product decision such as G1-A. Recommendations remain distinct from user approval.

## Review comprehension before standardizing

Judge the prototype using these questions:

1. Can the reader point to what runs in parallel without operating a control?
2. Can the reader identify what reaches the endpoint and what stays local?
3. Can the reader compare the five failure outcomes without remembering a previous screen?
4. Is the missing step between paste dispatch and automatic learning visible?
5. Does a narrow screen preserve the causal relationship between each condition and its outcome?

Check keyboard focus, contrast, overflow, link targets, offline assets and no-script readability in the rendered page. Record the user's comprehension and preferred representation before changing the installed skill. Layout checks alone cannot establish that the explanation is easier to understand.

## Review artifacts

- [A: Flow map](a-flow-map/index.html) and [rationale](a-flow-map/rationale.md)
- [B: Storyboard](b-storyboard/index.html) and [rationale](b-storyboard/rationale.md)
- [C: Responsibility lanes](c-lanes/index.html) and [rationale](c-lanes/rationale.md)
- [D: Concise overview](d-overview/index.html) and [rationale](d-overview/rationale.md)

The review may choose one direction, combine useful parts, or request another round. No winner is recorded yet.
