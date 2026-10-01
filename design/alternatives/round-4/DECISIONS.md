# Combined design revision

## User-selected ingredients

Use J's compact overview with continuous connectors. The previous dashed-to-solid segment was a drawing error with no intended architectural meaning. Use H's explanation of the learning question, I's missing-completion illustration and option layout, and I's concrete request/response examples. Preserve the existing annotated app captures and their provenance.

Question headings contain their stable identifier and the question itself: “G1. What evidence permits automatic learning?” Remove the redundant case heading. Keep G1-A/B/C pending; these presentation choices do not settle the learning policy.

## Presentation changes

The user selected section-03 alternative A, “Follow the reply through Handy.” Section 03 now connects the illustrative reply, Handy's checks, stop conditions and output in one flow. The proposed effect is explained below that flow. Persistence and the next recording have a separate section. This presentation decision does not approve any G1 product option.

Use restrained headings and remove introductory status pills. If a badge or color key is introduced later, it must identify a class used consistently in the actual content. Keep evidence qualifications at the affected illustration, capture or claim, with detailed limits in the evidence section.

Target 1440px of content on wide screens, with modest outer gutters and responsive reflow on narrower screens. Subtitles fill the available content width. Section titles sit above their descriptions so a tall, narrow title column cannot create empty space beside a short description. Keep useful side notes beside examples. Use square containers for examples, diagrams and decision options.

## Scope and sources

The user clarified that show-me may create an interactive app, including multiple views and working controls. A single standalone page is the current artifact's format, not a limit on the skill. Earlier feedback about hidden dropdown content means that interaction must communicate consequences and support comparison; it does not prohibit interactive experiences. This clarification is carried into the generic skill proposal. It does not request a new application implementation for this architecture review.

The contents index occupies a separate 200px left column and can collapse to a 48px rail. The reading pane independently scrolls and centers its 1440px content area with outer gutters. At widths that cannot fit the sidebar and full content together, content reflows within the available second-column width. On small screens the index expands over the content from its rail; it starts collapsed when JavaScript is available. The native disclosure remains operable without JavaScript. The earlier horizontal navigation is replaced by this index.

This revision combines the retained parts of round 3; its builder reads the previous I page, J diagram and selected section-03 alternative A without modifying the source alternatives. The H question description is carried into the builder. Application behavior and installed skills are unchanged. Generic proposed steering is in SKILL-PROPOSAL.md. Captures are historical native UI with synthetic data/events; the request/response example is illustrative, not runtime proof.

Regenerate with `uv run design/alternatives/round-4/build.py`. Verification is recorded in TASKS.md and proof/ after browser review.
