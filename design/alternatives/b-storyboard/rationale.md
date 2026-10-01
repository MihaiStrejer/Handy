# Alternative B: the session storyboard

The hypothesis is that a concrete utterance, shown across simultaneous frames, makes the parallel work and output boundary easier to explain than a stepper. The reader can point at the spoken phrase, its dictionary entry, the request, and the intended output without remembering a hidden previous step. Five always-visible exception strips repeat the same cause, blocked stage, and experience columns so cancellation, focus, field changes, invalid replies and protected input can be compared directly.

The three comparison moments are frame 01 (audio and capture run alongside each other), frame 03 plus its expanded message bodies (system instructions and user data), and frames 04–06 plus the exception strips and G1 (validation, dispatch, and unfinished learning). The code-derived Terminal example uses unavailable selection. The field-change strip explicitly changes to a supported native Edit control; it does not suggest Terminal selection capture works.

The page retains P1–P7 and G1-A/B/C, the existing palette, and the recommended readback option. Orange marks the unresolved learning boundary. None of the drawings claims to be a screenshot or completed voice run. Source excerpts include line numbers and file hashes. Original app code, architecture pages and installed skills are untouched.

## Tradeoffs

The storyboard is longer than a compact diagram, and its fixed example covers fewer destinations than an exhaustive scenario browser. It exposes causality and exceptions without interaction. Source excerpts remain collapsible because inspecting implementation is optional; all behavior needed to understand the story remains visible. At mobile widths, frames and exception cells stack with their labels intact. A print view preserves the story but omits source excerpts.

The main potential misunderstanding is treating the pictured Terminal output as observed completion. The frame labels it as an intended result, and the next frame puts the missing completion evidence next to learning. A human review should check whether this distinction survives a quick reading.

## Proposed show-me skill rules

These are proposals for a later skill revision; no installed skill is changed.

1. Choose the explanatory form from the reader’s question before applying a page skeleton. For a temporal process, start with one concrete input and its successive states. For comparisons, preserve a shared coordinate system across scenarios.
2. Show the key states needed for comparison simultaneously. A Next button, tab or dropdown may add detail but must not be the only way to discover a material failure, gap or boundary.
3. For concurrent work, place the lanes beside each other and name the point where their results meet. Avoid serial step numbers that imply the microphone waits for context capture.
4. For every material exception, show cause, blocked stage and user consequence. Name the supported platform/control boundary in the same row. Distinguish a request that was never sent from a sent request whose result is discarded.
5. Carry one small synthetic example through inputs, serialized roles and intended outputs. Show exact message fields where role separation matters; mark abbreviated contracts explicitly.
6. Put a missing capability at the point where the story needs it. Do not reserve unfinished learning for a distant warning list while drawing a complete learning loop above it.
7. Distinguish code-derived behavior, observed behavior and proposed behavior beside each visual. A screenshot of the explainer validates layout only, never the application flow it depicts.
8. Review comprehension with three prompts: what happens in parallel, what reaches the model, and why dispatch cannot yet authorize learning. Record misunderstandings before standardizing the presentation.

## Reproduction and limits

Run `uv run design/alternatives/b-storyboard/build.py` from the repository root. It regenerates `index.html` from the inline template and embeds current source excerpts. The output has no network assets. Evidence is tied to the current working tree, which includes uncommitted implementation beyond the stamped baseline commit. This alternative does not rerun Handy’s runtime tests or verify a live voice session.

Rendered with installed Microsoft Edge through Playwright at 1440 × 1000 and 390 × 844. Both sizes have six visible story frames, five visible exception strips, no horizontal document overflow, no missing internal anchors and no external assets. Source links open their evidence panels; no page errors occurred. The desktop and mobile full-page screenshots were inspected for layout. Run `node design/alternatives/b-storyboard/verify.mjs` to repeat these checks. Screenshots prove the explainer renders, not that Handy completed a voice session.
