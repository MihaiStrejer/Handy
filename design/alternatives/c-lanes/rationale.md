# Alternative C: responsibility lanes and data lifetimes

The hypothesis is that keeping owners fixed while time moves downward will help the reader see concurrent capture and audio, their join, and which component stops output. A separate lifetime table answers what survives the next utterance without asking readers to infer persistence from arrows.

The page has five lanes: user and widget, audio, session and context, endpoint, output and storage. P1–P7 and G1-A/B/C keep their existing meanings. The comparison moments are the sequence, role-separated request data, and a visible exception matrix followed by the learning boundary. All five exceptions appear together. Source excerpts may be expanded because they support the explanation rather than contain hidden branches.

This structure helps readers answer who owns an action, what runs alongside it, and what carries forward. It is less suitable for a quick product tour or learning exact UI controls. The wide diagram scrolls locally on a phone with a visible swipe hint; exception and lifetime rows stack with their column labels, as do request and decision cards. No optional lane focus was added because the whole sequence must remain legible at rest.

The diagrams and widget are synthetic, and the full voice run remains explicitly unverified. The page does not reuse native captures because UI evidence would distract from its ownership question. Automatic learning stays pending. G1-A recommends verified readback; it does not imply that readback is implemented or approved.

Proposed show-me rules, for review only:

- Choose the visual from the reader’s question: sequence lanes for timing and ownership, a state diagram for permitted transitions, a matrix for outcomes, and a lifetime table for persistence.
- Keep every material branch visible in the initial document. A selector may emphasize a branch, but must not be the only way to discover it.
- Put exceptions beside the owner of each check and its output consequence. State support limitations beside the relevant check.
- Mark parallel work and its join explicitly. A simple sequence must not imply that background work blocks another operation.
- Separate configuration snapshots, session data, saved history, compatibility caches and conversation state. Include a next-operation column when explaining reuse.
- Give each diagram one job. Reuse stable names and IDs across request, lifetime and decision views.
- Show missing completion evidence before asking for the policy choice that depends on it.
- Check the no-interaction reading path: main flow, every known exception, pending choice and verification limits must be discoverable without operating a control.
- Verify desktop and mobile rendering; contain unavoidable horizontal scrolling locally and provide a complete diagram text alternative.

Regenerate with `uv run python design/alternatives/c-lanes/build.py`. The builder embeds current source excerpts and stamps the Git baseline. The parent is checking browser rendering across alternatives. Application behavior requires separate live verification.
