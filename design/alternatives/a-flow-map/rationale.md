# Alternative A: one visible flow

## Design hypothesis

Readers can explain the system more accurately when runtime order and failure consequences share one continuous diagram. The main path runs downward; each side branch appears at the stage that enforces it. Cancellation spans the active session, so it receives a visible cross-cutting strip before the parallel lanes. No control hides a failure or the missing learning boundary.

## What this simplifies

The page replaces the original overview, walkthrough, request explanation and failure dropdown with one flow. Parallel capture and audio share a split-and-join shape. System instructions and user data occupy adjacent boxes at request assembly. Response checks, changed fields and moved focus appear beside their own gates. The missing completion evidence appears between dispatch and learning, directly before G1 options. A reader can follow the whole story without selecting a scenario or remembering an earlier screen.

The same palette and P1–P7 / G1-A/B/C identifiers remain. The prototype uses code-derived illustrations and source links. It makes no claim to be a native app screenshot or a completed runtime demonstration. G1-A remains a recommendation; the answer remains pending.

## Tradeoffs

The vertical flow needs scrolling, particularly on a phone. “Always visible” means present in the document with no interaction required, rather than everything fitting in one viewport. On narrow screens, branches sit immediately below their causal node. Dense source excerpts and UI captures are omitted to preserve the runtime explanation; source links retain auditability. This is less suitable for comparing several complete destination scenarios side by side.

## Proposed show-me instruction changes

These are proposals only; the installed skill was not edited.

1. Start with the question the reader needs to answer, then choose a diagram that makes its relationships visible. A behavior explanation may use one continuous flow instead of repeating “whole,” “pieces” and “reference” sections.
2. Require the normal path, material failure branches and unresolved boundary to be readable in the initial document. Do not put them behind dropdowns, tabs, step controls or collapsed details. Interaction may highlight already visible content.
3. Attach a failure to the gate that detects it. Place a cross-cutting condition, such as cancellation, across the active scope and describe its final guard. Do not imply that a cancellation immediately stops all background work.
4. Distinguish runtime flow, data roles and evidence. Use split-and-join lanes for concurrency, separate instruction/data boxes at a request boundary, and an explicit broken boundary for missing behavior.
5. At mobile width, preserve each branch next to its causal stage in document order. Test overflow and initial visibility with scripts disabled.
6. Keep stable piece and option IDs in the diagram and decision cards. Keep recommendations separate from recorded answers. Place the decision immediately after the missing behavior that makes it necessary.
7. Label code-derived diagrams separately from observed captures and executed tests. Cite concrete source locations; state the unverified runtime claim once in a visible evidence section.

## Verification

Sources inspected: `src-tauri/src/context_profiles/{mod,request,session,routing}.rs`, invocation and output sections of `actions.rs`, initialization references in `lib.rs`, and the original architecture template. Browser checks are recorded after rendering. No application code or installed skills were changed.

Browser validation: `node design/alternatives/a-flow-map/check.cjs` passed in installed Microsoft Edge with JavaScript disabled at 1440 × 1100 and 390 × 844. Both widths had no horizontal overflow; four local branch cards were rendered, cancellation remained visible in document flow, there were no select controls, all internal anchors resolved, and no page errors occurred. `desktop.png` and `mobile.png` were rendered and visually inspected. These checks validate this explanatory HTML only, not Handy's live voice pipeline.
