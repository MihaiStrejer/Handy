# Architecture explanation alternatives

## Purpose

Help the reader understand Handy's implemented voice-session architecture and its unfinished feedback path. The user found that a dropdown containing failure cases hid the information needed to understand the system. This round explores changes to explanation structure and diagram behavior, using the current architecture as common content. It does not redesign Handy's application UI or commit changes to the installed show-me skill.

## Scope and reading flow

Scope: the architecture explainer and its reusable presentation rules. Mode: read. Keep the existing palette and terminology so differences come from how the information is organized.

The reader opens a comparison index, examines each alternative, follows the visible voice-session flow, compares exceptions at the relevant stages, and reviews the proposed skill rules. Selection remains pending. The index links to separate offline pages; each page keeps its primary explanation visible and puts source detail after it.

## Common content

1. Startup prepares managers, persisted settings and process-local context state.
2. A post-processing invocation with both flags enabled starts a new session. Context capture and audio run alongside each other.
3. Routing resolves a profile and freezes its effective prompt, dictionary, memory and captured input.
4. The transcript and context become one system message plus one user data message through the existing endpoint.
5. Strict reply validation, supported-field recheck and destination focus checks precede existing output dispatch.
6. The next utterance starts a fresh request. Model feedback does not yet populate the dictionary or memory.

Compare all alternatives at three moments: concurrent audio/context capture; the two-role request; output and exceptions, including the missing learning boundary. All five exceptions must be discoverable without selecting a menu item: cancellation, focus change, changed field/selection, invalid reply, and protected input.

## Alternatives

| ID | Representation | Reading question |
| --- | --- | --- |
| A | Flow map with attached exception branches | Where does the process go, and where can it stop? |
| B | Concrete session storyboard with exception strips | What happens to my words and my selected text? |
| C | Responsibility lanes and data lifetimes | Who owns each step, and what survives? |
| D | Concise overview with visible outcome comparisons | What is the smallest explanation that retains the important relationships? |

## Evidence and boundaries

Current code and the existing architecture walkthrough supply facts. Diagrams illustrate code behavior; they are not recordings. Keep completed implementation, targeted verification, deferred workspace capture and unverified full voice execution distinct. Retain P1–P7 and G1-A/G1-B/G1-C when discussing the existing task/decision structure. G1 remains unanswered; the separate choice of presentation A–D does not resolve it.

Sources: `../context-profiles-architecture.template.html`, `../../src-tauri/src/context_profiles/`, `../../src-tauri/src/actions.rs`, `../../plan.md`, and captures under `../proof/architecture/`.

## Review criteria

- Can the primary flow be understood without clicking?
- Do arrows, alignment and proximity explain causality, concurrency or ownership?
- Are failure causes shown beside their consequences?
- Does an example clarify the request without implying a capability that is absent?
- Does the page work at desktop and narrow widths, with keyboard focus and readable text?
- Does each proposed skill rule explain when to use a representation and when to avoid it?

The final direction is the user's choice. A combined direction or another round remains possible.
