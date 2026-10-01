# Round 2 tasks

| Task | Owner | Status | Evidence |
| --- | --- | --- | --- |
| E: purpose annotated lanes | GPT-6-Sol / round2_purpose | Complete | Parent checked source claims, rendered arrows and rationale; desktop/mobile/no-script results in proof/e-purpose-lanes.json. |
| F: aligned two-run comparison | GPT-6-Sol / round2_reuse | Complete | Parent corrected payload field names, source links and authority wording; desktop/mobile/no-script results in proof/f-two-runs.json. |
| G: map and boundary closeups | GPT-6-Sol / round2_boundaries | Complete | Parent reviewed parallel-start representation and current/proposed distinctions; desktop/mobile/no-script results in proof/g-boundaries.json. |
| Cross-project instruction review and comparison index | Root | Complete | Generic proposals reviewed separately from app examples; comparison index passes links, image loading, overflow and no-external-request checks in proof/index.json. |
| Carry forward annotated captures, overview and terminology feedback | Root | Complete | Shared draft and all three proposals include generic language/evidence rules. Index includes reused annotated native captures and an overview with concrete action labels; proof/index-retained-desktop.png and proof/index-retained-mobile.png. |

Prior preference: C communicates ownership more clearly. Final direction and installed-skill revision remain undecided.

Verification: `node design/alternatives/round-2/verify.mjs` checks candidate pages; `uv run design/alternatives/round-2/build-index.py` regenerates the index; `node design/alternatives/round-2/verify.mjs index` checks the index. Pages rendered in Edge with JavaScript disabled and made no external network requests. No new app capture, live voice run or provider-cache measurement was performed. User comprehension and presentation choice remain for review.
