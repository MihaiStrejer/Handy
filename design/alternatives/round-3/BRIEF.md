# Round 3: architecture a newcomer can follow

## Reader feedback and contract

The user rejected E, F and G as overall designs. C communicated more of the architecture clearly. The original overview flow and annotated captures of the actual app remain strengths. E's flow was useful but not an improvement over the original. F's open question sections, specifically G1 learning after output, should be preserved. The next round must improve explanation rather than merely rearrange cards or focus the whole page on caching.

The reader wants to understand the app from startup through a voice session, the request and output, and what carries into a subsequent session. Labels such as “Managers” and “Context lane” failed because they mixed code terminology with diagram groupings. Use established project/user terms; explain necessary new terms where first encountered. A visual group must not appear to be an actual component or function unless source evidence establishes that identity.

## New alternatives

| ID | Agent | Brief |
| --- | --- | --- |
| H | GPT-6-Astra | Develop C into a connected architecture map: a readable overview and carefully linked details, with UI evidence connected to the actions it explains. |
| I | GPT-6-Sol | Build an illustrated walkthrough of one concrete voice session, making the configuration, captured context, request and output easy to follow. |
| J | GPT-6-Luna | Build a newcomer explanation organized around the essential questions, with a compact whole-system flow and concrete evidence answering each question. |

Different models and editorial briefs are an exploration, not a model quality benchmark. All three must cover the same whole-system path, annotated native captures, explicit two-message request purpose, visible failure consequences, and F-style G1 options. No winner is selected in advance.

## Files and boundaries

Workers own only their assigned folder: `h-connected-map/`, `i-illustrated-session/`, or `j-reader-questions/`. Each produces standalone `index.html`, `rationale.md`, and `SKILL-PROPOSAL.md`; an optional local builder is allowed. Parent owns the comparison index, verification and tracker. Do not edit application code, original alternatives or installed skills. Do not revert other work. If a source or tool is unavailable, report it and keep dependent claims unverified.

## Sources to read

- `design/context-profiles-architecture.template.html`: original content and source IDs; generated sibling HTML includes excerpts.
- `design/alternatives/c-lanes/template.html`: preferred first-round ownership design.
- `design/alternatives/round-2/f-two-runs/index.html`: preferred G1 presentation. Preserve G1-A/B/C meanings and pending status; do not imply that F was selected overall.
- `design/alternatives/round-2/GENERIC-INSTRUCTIONS.md`: portable draft including latest terminology and capture feedback.
- `src-tauri/src/lib.rs`, `src-tauri/src/actions.rs`, `src-tauri/src/context_profiles/{mod,request,session,routing,storage,capture}.rs`, `src-tauri/src/llm_client.rs`: inspect relevant code claims.
- `design/proof/architecture/dictionary-annotated.png`, `context-original.png`, `overlay.png`, `capture.json`: real existing native captures. Embed as base64; retain provenance and synthetic data/event caveats. These are historical captures, not a new live test. View before using.

## Architecture facts to preserve and verify

Profiles apply to a post-process invocation when post-processing and profiles are enabled. The original destination and configuration are captured for that recording. Audio recording and context capture proceed independently and meet before request assembly. App routing is implemented; Windows Terminal workspace detection remains deferred. Native Windows Edit selection is supported; general browser/terminal selection is not. Settings selection chooses a profile to edit, not the runtime route.

Current requests send two messages every time: system instructions and reply contract; user data containing dictionary, short_term_memory, input_context and transcript. This separation expresses instructions versus data, not a claim that one message is always stable and the other always changes. No continuing provider conversation is maintained. Possible provider reuse of an unchanged prefix is an optimization proposal; explicit prompt-cache control and cached-usage measurement are absent. The implemented compatibility cache remembers a synthetic endpoint contract check, not model prompt reuse.

Reply validation, session cancellation, supported field recheck and final focus checks precede output. The endpoint proposes text/operation/effect; successful dispatch does not establish that the destination actually contains the text. Automatic learning is missing. Preserve G1-A readback recommended, G1-B ask user, G1-C trust dispatch, all pending. No live voice end-to-end or cache benefit claim is justified by these pages.

## Design and verification

Keep the established blue/neutral/orange visual language, but vary explanatory composition substantially. Keep headings and diagram labels readable at normal desktop scale. Core facts must be visible without dropdowns, hover or a stepper. Optional source details may collapse. Use purposeful whitespace, actual annotated captures and clear connectors; avoid a wall of same-sized cards. Distinguish code-derived illustrations, captured UI and proposed behavior where they appear.

Review `index.html` at desktop and mobile, without external network assets or required JavaScript. Use installed Edge through Playwright with Node; Python commands must use uv. Never launch bare python/pip, use Node to parse JSON, or use PowerShell ConvertFrom-Json. No installations, app manipulation, API requests, commits or skill installation. Parent can render if a worker cannot; do not call unperformed rendering checks passed.

## Portable skill proposals

Each proposal must steer explanations across projects and domains. No mandatory app concepts, providers, message roles, dictionaries, caching, fixed lane count, or fixed page skeleton. Keep app-specific examples in the rationale. Include one unrelated hypothetical transfer example there. The skill should guide selection of a representation from reader need, terminology, evidence and meaningful relationships. User choice remains pending.
