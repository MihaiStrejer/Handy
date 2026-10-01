# D: concise overview with visible outcome comparisons

## Hypothesis

A reader can explain the architecture after one desktop viewport if the page preserves three relationships: context capture runs alongside audio, two message roles meet at one endpoint, and output dispatch has no implemented learning return path. Five small diagrams below the main flow make the exceptions comparable without changing page state. The page targets a 1440 × 1000 viewport; narrow layouts stack the same content in reading order.

## Tradeoff

This direction gives up a detailed tour of every component and the settings UI. Each panel explains one causal relationship, with source detail below. It works for initial orientation and comparing consequences. It is less suitable for tracing every owner or diagnosing a specific lifecycle race; the responsibility-lane alternative better serves that task. The request example is synthetic and explicitly limited to native Edit selection support.

## Concrete proposed show-me changes

- Add an overview mode before applying the six-section skeleton: for an architecture explanation, choose at most three relationships the reader must understand and draw those before reference material. Keep full decision cards below when decisions remain open. Avoid this mode when the task requires exhaustive migration parity.
- Replace the fixed two-column shell default with a content decision: use a full-width reading surface when the core artifact is a flow or comparison; retain navigation when many independent features need repeated access. Never shrink the main diagram solely to preserve a sidebar.
- Require simultaneous visibility when cases share a decision boundary. Use small multiples with a concrete input change and visible consequence. Menus may emphasize an already visible case, but must not hide required cases. Avoid small multiples when dozens of states would make each illegible; group cases by gate first.
- Require a concurrency fork and join whenever parallel work matters. A left-to-right list of cards is insufficient if it implies serial execution. Use arrows and spatial alignment for this relationship; use prose for incidental timing.
- Show instruction roles separately from request data when explaining model calls. Include a small concrete example and identify synthetic values. Keep endpoint compatibility, prompt caching and conversation persistence separate claims.
- Place missing behavior in the diagram where the connection is absent. Draw the dispatch-to-learning gap with a dashed boundary and link to the existing stable decision ID. A status badge alone does not explain why the path is incomplete.
- Move source excerpts after the first explanation while retaining compact links beside claims. Keep unavailable behavior and unverified execution visible near the diagram. Source inspection supports the code diagram; it does not establish live output completion.

## Evidence and verification

Sources inspected: the common brief, original architecture template, project AGENTS.md, context_profiles/request.rs, session.rs, mod.rs, capture.rs, lib.rs and the recording/output sections of actions.rs. G1-A remains the recommendation for verified field readback; G1-B and G1-C retain their existing meanings. No decision is marked answered. The learning write path and workspace capture remain absent/deferred. No full voice execution is asserted.

Parent browser checks passed at desktop and narrow widths, with scripts disabled and network access blocked. Static checks found five SVG diagrams and 27 links; source paths and internal anchors resolved, with the comparison index pending creation at the time of that check. Parent source review confirmed the content and corrected one excerpt line reference. This slice edits only index.html and rationale.md in d-overview; it changes no application or installed skill files.
