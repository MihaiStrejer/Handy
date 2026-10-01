# Proposed generic refinements to show-me

Review draft; no installed skill change. These refinements supplement the prior portable guidance about evidence, terminology, diagrams, captures and visible decisions.

## Choose the experience, not a fixed document format

Show-me can produce an interactive app, not only a single explanation page. Choose the scope and interaction model from what the reader needs to understand, explore or decide. A focused page may suffice; other tasks benefit from multiple views, navigation, scenario controls, state simulations, editable examples, comparisons or decision capture. Do not impose a single-page, static or no-JavaScript constraint by default.

Use interaction to expose meaningful behavior or relationships. Make the initial view understandable, label controls by what they change, and show the resulting consequences clearly. Keep information needed for a direct comparison visible together; avoid making users remember separate dropdown states to compare them. This is a comprehension rule, not a prohibition on interactive exploration or progressive disclosure.

For an interactive app, verify its navigation, state changes, keyboard behavior and relevant user journeys. Distinguish simulated behavior from real integrations. Choose packaging, persistence and offline support to fit the delivery context, and preserve evidence and decision identifiers across views. A static fallback is useful when required by that context; it is not a universal requirement.

## Keep the hierarchy proportional to the explanation

Use concise, restrained page and section headings. A decision heading should contain its stable identifier and the actual question. Remove secondary headings that repeat the same situation without adding information. Put the description immediately below the question, followed by the situation illustration, options, tradeoffs and answer state when those help the decision.

Use badges and color only to communicate a meaningful category or state that the reader can locate consistently in the content. A legend must explain an encoding actually used by the page. Avoid decorative rows of status pills in the introduction. Preserve important evidence limits beside the claims they qualify and in the supporting evidence section.

## Make connector styles meaningful

Use continuous connectors for a continuous relationship. A change of line style, color or arrow shape must have an intentional meaning that is explained and applied consistently. Inspect complete paths through branches and joins; a stylistic seam must not suggest a different kind of dependency. Mark missing or proposed connections explicitly when the evidence requires it.

## Use the available reading area

Aim for a 1440px content area on wide desktop screens, adapting to the available viewport with modest gutters. Let subtitles and short explanatory paragraphs use their allotted width. Avoid narrow heading columns that force several lines and leave empty space beside short descriptions. Keep code samples, annotations and comparison options aligned with the content they explain; reflow on small screens without changing the relationship. Use square containers for diagrams, examples and decisions in this design direction.

For a long explanation, place its contents index in a separate collapsible column, approximately 200px wide, outside the reading area's width budget. Center the intended content width within the remaining column. Give the index and reading pane independent scrolling, keep the collapse control accessible, and preserve anchor navigation when either pane scrolls. On narrow screens, reflow the reading area and use a collapsed navigation rail or overlay so navigation does not leave an unusably narrow text column. Derive the index hierarchy from the actual content and avoid duplicating it in another navigation bar.

## Show concrete examples at important interfaces

When understanding a boundary depends on the actual information exchanged, show representative inputs and outputs in the system's real format. Explain their purpose, ownership and relevant fields next to the example. Include response examples when the result controls later work. Label synthetic or abbreviated content and preserve authority and validation rules. Use the appropriate format for the domain; do not require JSON, network calls or a particular provider in every explanation.

## Review the combined result

For a section explaining execution, follow the result through the receiving component, its acceptance checks and the resulting action. Attach failure consequences to the checks that cause them, and keep the successful path visible. Put representative data beside the point where it enters the flow. Explain any information used for a different purpose separately, and give persistence or the next operation its own section when it answers a different question. Use this form when the reader needs to understand what happens next; do not impose it on every explanation.

Retain the representations that the reader found useful and revise the observed sources of confusion. Check visual connections, terminology, screen captures, examples, question hierarchy, text width and whitespace at the intended reading sizes. Preserve uncertainty without letting repeated disclaimers dominate the main explanation. Rendering checks do not establish reader comprehension or live system behavior.
