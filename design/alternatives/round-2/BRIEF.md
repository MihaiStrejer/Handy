# Round 2: explain the purpose of the architecture

## User feedback carried forward

The user found alternative C's responsibility lanes clearer than the other first-round designs. They still had to infer why the request separates instructions from changing data and whether caching was implemented. This round retains visible ownership and handoffs while exploring how to communicate rationale, conditions and status. Three GPT-6-Sol agents produce independent candidates.

## Scope

Additional reader feedback: preserve annotated captures of the real app and a compact architecture overview. The labels “Managers” and “Context lane” required the reader to infer both implementation vocabulary and diagram vocabulary. Check established project terminology, explain necessary new terms at first use, and distinguish actual components from groups of steps invented for the explanation. Carry this forward as generic guidance; Handy-specific replacement labels belong only in the example pages.

Create explanation prototypes for review. Handy is the test case. Any proposed installed-skill instructions must be generic across projects, solutions and functionality. Do not change application behavior, install a skill revision, or record a final presentation choice during this round.

Keep the original alternatives for comparison. New presentation IDs are E, F and G; these are separate from product decision IDs such as G1-A.

## Shared comparison moments

1. Who owns the work, what runs concurrently, and where results meet.
2. What crosses a boundary, why information is separated, and which parts change between operations.
3. Which behavior is implemented, which benefit is a proposed optimization, and which outcome has not been verified.

The primary reading must explain these moments without dropdowns, steppers or hover. Sources may sit below the explanation or in disclosures. Show material failures with their consequences, without letting repeated reference material dominate the page.

## Candidate directions

| ID | Lens | Hypothesis |
| --- | --- | --- |
| E | Purpose annotated lanes | Rationale beside each important handoff explains the design without requiring a second diagram. |
| F | Two aligned runs | Showing unchanged and changed information together explains reuse and prevents confusion with conversation persistence. |
| G | System map with boundary closeups | A small ownership map plus visible closeups makes important boundaries understandable without crowding the whole sequence. |

## Test-case facts

Current code sends one system message containing instructions and a response contract, plus one user message containing dictionary, memory, input context and transcript. Both messages are sent per operation. Dictionary data currently belongs to the user envelope. No explicit provider caching controls or cached-token measurements are implemented. Stable-prefix arrangement and deliberate provider reuse are proposed optimizations; their benefit is conditional and unverified. The compatibility cache records an endpoint contract check, not model prompt reuse. Automatic learning remains unimplemented. Existing selected-text support, workspace deferral and full voice-run verification limits remain as documented in the original walkthrough and actual code.

## Genericity gate

Each candidate includes a separate `SKILL-PROPOSAL.md` containing portable steering instructions. It must not mandate project names, particular providers, model roles, caching, dictionaries, a fixed lane count, or a fixed set of failures. It should tell an author how to identify relevant relationships, select a representation, explain rationale without inventing intent, mark status, and check comprehension. Domain-specific content belongs in the prototype and rationale, outside the reusable instruction text.

Each rationale applies its rules to one small unrelated workflow as a transfer check. These hypothetical examples test the proposed method and do not claim evidence about a real second project.

## Verification and decision

Inspect current source for factual claims. Render all pages in installed Edge at desktop and narrow widths; check offline assets, anchors, overflow and no-script reading. Review diagrams for causal meaning and visible distinctions between current and proposed behavior. User review determines which direction to retain, combine or revise. Browser checks do not establish comprehension.
