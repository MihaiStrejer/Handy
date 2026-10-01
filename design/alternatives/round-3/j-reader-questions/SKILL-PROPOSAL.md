# Show-me skill proposal · review draft

This is a portable draft for a possible addition or revision to the existing `show-me` skill. It does not define a separate installed skill.

Use this skill when the task is to help a new reader understand how a system works across its important parts. The result may be a page, document, diagram, or another format suited to the reader. Do not assume a fixed system type, set of components, diagram shape, or page outline.

## Establish the reader’s need

Identify who will read the explanation, what they already know, and what they need to understand or decide. Gather the request, existing documentation, relevant source code or configuration, observed interfaces, and supplied captures. Prefer first-hand evidence over summaries. If a required source is unavailable, identify the dependent claim and leave it unresolved rather than filling the gap from guesswork.

Write down the system’s meaningful beginning, main work, handoffs, outcomes, and what remains available afterward. Include failure or stopping conditions when they change what the reader should expect. Keep a boundary visible wherever data, authority, responsibility, or state changes owner.

## Choose a useful explanation

Choose the organizing questions, sequence, and visual form from the reader’s need and the relationships in the evidence. A compact overview may orient readers before focused explanations. Use a flow, timeline, state diagram, map, table, comparison, or annotated interface where it makes a relationship easier to see. Join parallel paths where they actually meet. Do not suggest that a grouping is a real component unless the evidence supports that identity.

Keep the essential facts available in the main explanation. Avoid hiding required understanding in interactions such as hover states, collapsed panels, or step controls. Use links or optional source details for material that supports review but interrupts the main path. Avoid repeating equal-sized cards when a connected diagram or varied layout better expresses the relationships.

## Write from evidence

Name real actors and actions. Keep project terminology when it identifies an actual setting, API, or behavior; explain an unfamiliar term when it first matters. Use plain words for relationships that do not have established names. Mark the difference between implemented behavior, observed behavior, inference, and proposal.

For information leaving or entering a boundary, show its contents and purpose where the evidence permits. Explain what can prevent an action and what a reported success actually confirms. State relevant limitations next to the claims they qualify. Do not turn a possible optimization, intended behavior, or design choice into a current guarantee.

For screenshots, use real captures when supplied or authorized. Preserve their source and context, distinguish fixtures from production data, and add visible annotations only when they help locate the detail being explained. Do not present a mockup or code-derived drawing as an observed interface.

## Review the result

Follow the main path from its start through its outcome and any meaningful next use. Check that each important claim has supporting evidence, that readers can distinguish current behavior from proposals, and that diagrams do not introduce unexplained terms or false ownership. Open the produced artifact at desktop and narrow widths when browser review is available. Check embedded media, navigation, labels, and links. Report any view or behavior that could not be checked.

Keep the result as short as the reader’s task allows while preserving the facts needed to understand the whole path. Do not impose a fixed number of questions, sections, cards, lanes, components, or visual conventions.
