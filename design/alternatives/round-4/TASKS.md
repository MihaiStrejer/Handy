# Combined revision tasks

| Task | Status | Evidence |
| --- | --- | --- |
| Add independent collapsible contents column | Complete | 200px sidebar, 48px collapsed rail, centered 1440px content at wide sizes; keyboard toggle, anchor navigation, independent scrolling and mobile overlay verified. proof/verification.json and sidebar screenshots. |
| Combine selected H/I/J parts | Complete | build.py and index.html |
| Record generic presentation rules | Complete | SKILL-PROPOSAL.md; installed skill unchanged |
| Desktop/mobile visual and link review | Complete | proof/verification.json; screenshots inspected for overview, question, request, response and mobile layout |
| Apply selected section-03 flow | Complete | A integrated at #after; persistence moved to #next-recording; decision and generic skill draft updated. Browser checks passed and new section screenshots inspected. |
| Explore clearer presentations for section 03 | Complete | section-03/index.html provides flow, annotated-reply and condition-table alternatives; section-03/NOTES.md records rationale and browser checks. A selected by the user. |

Product decision G1 remains pending. This task changes explanation artifacts only.

Verified in Edge with JavaScript disabled at viewport widths 1920, 1688, 1440, 1200, 900 and 390. At 1920, the content and section subtitle both measure 1440px, centered in the second column; the sidebar separately measures 200px. Keyboard collapse/expand and independent scroll positions passed. A separate mobile check with JavaScript enabled confirmed the initially collapsed rail and expanded overlay. Checks confirm no status pills, solid overview connectors, square example/decision containers, the requested G1 heading, no redundant case heading, loaded images, resolved links/anchors and no document overflow or external network requests. The first screenshot pass had a locator error for the response section; the corrected verifier passed. No application or endpoint behavior was retested. Regenerate with `uv run design/alternatives/round-4/build.py`; verify with `node design/alternatives/round-4/verify.mjs`.
