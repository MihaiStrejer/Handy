# Round 3 tasks

| Task | Owner | Status | Dependencies / evidence |
| --- | --- | --- | --- |
| H connected architecture map | Astra | Complete | Parent reviewed source claims, map, desktop/mobile and G1. Condensed hero and map; evidence in proof/h-connected-map.json. |
| I illustrated voice session | Sol | Complete | Parent reviewed source claims, concrete example, desktop/mobile and G1. Corrected protected-input consequence, startup condition and mobile hero padding. Evidence in proof/i-illustrated-session.json. |
| J reader questions | Luna | Complete | Corrected disconnected overview arrows, labels and G1 width; parent reviewed final diagram and options. Evidence in proof/j-reader-questions.json. |
| Content and generic proposal review | Parent | Complete | All three proposals checked for cross-project guidance; each rationale contains an unrelated hypothetical transfer example. No installed skill change. |
| Browser verification and comparison page | Parent | Complete | All candidate pages and comparison index checked; final I proof and embedded preview refreshed. Index desktop screenshot inspected. Evidence in proof/index.json. |

No presentation choice or installed-skill update is approved. The user retained the original overview and annotated captures, C's clarity, and F's question sections. E/F/G remain rejected as whole-page directions.

## Verification

`node design/alternatives/round-3/verify.mjs` checks candidates in installed Edge at 1440 × 1000 and 390 × 844 with JavaScript disabled. It checks document overflow, embedded image loading, local links, internal anchors, visible required topics and absence of external network requests. Parent additionally inspected actual screenshots and corrected visual semantics that these checks cannot establish. The index is generated with `uv run design/alternatives/round-3/build-index.py` and checked with `node design/alternatives/round-3/verify.mjs index`.

These are explanation prototypes. Native app images are reused historical captures with synthetic data/event caveats; this round performed no new live voice, endpoint caching or output-delivery test. Reader comprehension and the presentation choice remain for user review. G1-A/B/C are the same pending product decision, independent of H/I/J presentation selection.
