# History processing details: design round 1

Status: archived comparison. The user selected Option B; see the [refinement and failure samples](round-2.html). Scope: History entry component and its detail surface. The [behavior specification](../components/history-post-processing.md) defines the same data and states for every direction. Open the [interactive board](round-1.html).

All directions inherit Handy's sidebar, system font, compact controls, dark background `#2c2b29`, text `#fbfbfb` and pink accent `#f28cbb` from `src/styles/theme.css`. They preserve audio playback and the existing history actions. Light mode uses the paired project tokens. No app-wide visual redesign is proposed.

## A: Inline details (recommended)

Thesis: keep a small metadata line under each recording and expand details inside the entry. The history list stays visible and scroll position remains meaningful.

First viewport: original transcript, audio controls and a wrapping summary with a Processing details button. Open state: Overview, Requests and Calls tabs expand underneath. Stress state: a failed run shows a short reason and unknown metrics; an older run says Details not recorded. Long request content has its own bounded text area.

Signature interaction: expand one entry, inspect or copy a request, collapse it without leaving History. Honest risk: an expanded entry pushes later entries down. Keep one entry expanded by default and restore the trigger on collapse.

## B: Side inspector

Thesis: keep the list stable while the selected recording's details occupy a separate inspector in the same History pane.

First viewport: the same compact summary and a Processing details button. Open state: the inspector has Overview, Requests and Calls tabs plus a close action. Stress state: failures and older records use the same inspector with absent values labelled explicitly. At narrow widths the inspector fills the History content area and the sidebar remains available.

Signature interaction: select successive entries while retaining the selected detail tab. Honest risk: Handy's starting window cannot show both a readable list and a wide request viewer, so the list is covered in compact mode.

## C: Detail page

Thesis: dedicate the History content pane to one run when inspecting it, providing the largest readable request area.

First viewport: the same history entry and compact summary. Open state: Back to history, run identity and the three detail tabs replace the list. Stress state: failures show call-level outcomes; older entries explain what was not retained.

Signature interaction: open a run, read its requests, return to the prior list position. Honest risk: comparing two recordings requires repeated navigation. This is strongest when request inspection is frequent and long prompts are common.

## Review controls

The board lets each option switch between Success, Failed and Older entry, open/close details, switch Overview/Requests/Calls, inspect either the compatibility check or rewrite request, copy a message, and switch the whole board to light mode. Audio and existing history action icons are visual context, labelled as disabled sample controls. The board makes no provider requests and modifies no Handy settings or history. All amounts and prompts are synthetic; the fictional rates are disclosed in the spec and the Requests view.

Choose A, B or C, combine a named feature from two options, or request a fresh round. The choice commits layout behavior only; request archive and price-source decisions remain visible in the spec.
