# Architecture explanation design round

| Task | Owner | Status | Verification |
| --- | --- | --- | --- |
| A: causal flow map | Astra agent alternative_flowmap | Done | Source review and parent Edge desktop/mobile/offline/no-script checks passed |
| B: session storyboard | Astra agent alternative_storyboard | Done | Source review and parent Edge desktop/mobile/offline/no-script checks passed |
| C: responsibility lanes | Astra agent alternative_layers | Done | Source review and parent Edge desktop/mobile/offline/no-script checks passed; mobile exception/lifetime rows stack with labels |
| D: concise overview | Astra agent alternative_overview | Done | Source review and parent Edge desktop/mobile/offline/no-script checks passed; source reference corrections applied |
| Comparison index and proposed skill guidance | Root | Done | Index desktop/mobile/offline checks and all local links passed; rendered previews inspected |

No presentation direction is selected. Installed skill and application behavior remain unchanged in this design round.

## Verification

Parent checks: `node design/alternatives/verify.mjs` for the four alternatives, followed by `uv run design/alternatives/build-index.py` and `node design/alternatives/verify.mjs index`. Individual page runs were used while agents completed. Per-page JSON results and screenshots are under `proof/`. All five material cases remain in the document with JavaScript disabled; no alternative uses a select control. C retains a locally scrollable wide sequence diagram on narrow screens with a visible hint. These checks establish page rendering and discoverability, not user comprehension or live Handy execution.

All four Astra design slices are complete. Review of A–D and any subsequent installed-skill change remain separate user decisions.
