# Proposed portable instruction: explain a system through a concrete case

Status: proposal for review. This file does not change an installed skill.

## Establish the reader’s question and evidence

Identify what the reader needs to understand or decide, what terms they already know, and which relationships explain the answer. Read the implementation, relevant documents and available captures before asserting behavior. Mark whether each important claim comes from observed behavior, implementation evidence, a clearly labeled illustration or a proposal. State missing evidence beside the affected claim.

Choose one concrete case when a sequence of real decisions and handoffs will make the system easier to follow. Keep its values plausible and internally consistent. Label invented inputs and outcomes as illustrations; do not turn them into runtime proof. If one case cannot expose a consequential branch, add a brief second case or exception at that branch rather than forcing the first case to represent everything.

## Select a representation

Give the reader a compact whole-system view before detailed closeups when they need orientation. Show order with a sequence, concurrent work with parallel paths that visibly join, ownership with lanes, and persistence with a lifetime view. Combine views only when each answers a distinct reader question. Make arrows, containment and ordering mean one thing consistently. Do not present an editorial grouping as a real component unless evidence establishes it.

Use project and user terms for primary labels. Explain necessary technical words at first use, then keep their names stable. Keep source symbols in supporting evidence. Put a consequential separation next to the place it matters and say what crosses it, who handles each side and why the separation changes the outcome. Avoid repeating the same account in several card grids.

For an existing interface, place an actual annotated capture beside the action it shows. Embed assets when a standalone artifact is required, preserve capture provenance, and label synthetic fixtures and historical captures. A screenshot proves visible state at capture time; it cannot prove hidden processing or eventual delivery. Use a clearly labeled illustration where no capture exists.

## Show outcomes and uncertainty

Trace the common path through its result and into what happens on the next run or interaction when state carries forward. Attach failures to their trigger or detecting stage and state their consequence. Separate an accepted intermediate action from evidence of the user-visible outcome. Distinguish implemented behavior from a recommended change, and keep established option IDs and pending status for unresolved decisions. Show material options and tradeoffs without requiring hover, a menu or step navigation.

## Check the artifact

Inspect the rendered page at intended desktop and mobile sizes. Check source and navigation links, readable labels, image quality, contrast, overflow and whether the causal relationships survive reflow. Test whether a reader can find the whole path, explain an important boundary, identify where a failure stops progress and distinguish evidence from illustration. Record any behavior that code or screenshots leave unverified.
