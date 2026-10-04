# Profile system prompt control

Design update: [Profile processing messages](processing-messages.md) supersedes the inheritance and placeholder rules below for the next implementation. This document records the earlier control design; the current app still uses that earlier behavior.

Scope: One control group in Profiles → Context selection. The user can inspect the instructions that Handy will use for this profile, edit General's default, create a profile override, or return to inheritance. The control uses Handy's existing settings style and keeps the profile selector and three-tab navigation unchanged.

| State             | What the user sees                                                                 | Available action                                                                                                         |
| ----------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| General default   | `General default` source label and its effective template.                         | Edit default, then Save or Cancel.                                                                                       |
| Inherited profile | `Inherited from General` label and a read-only view of General's current template. | Customize prompt creates an editable copy; Save creates an override.                                                     |
| Custom profile    | `Custom prompt` label and this profile's effective template.                       | Edit prompt or Use General prompt instead. Reset removes the override.                                                   |
| Editing           | Editable multiline template, named variable buttons, Save and Cancel.              | Select a variable to insert it at the caret. Save validates and commits; Cancel restores the current effective template. |
| Invalid template  | Inline error beside the editor naming the problem.                                 | Correct the empty prompt or unsupported/malformed placeholder, then Save again.                                          |

General's template is the default source for every profile without an override. A change to General affects the next session for inheriting profiles; an active session keeps its frozen template. A specific profile's override remains unchanged when General changes. The selected provider remains in Post Process, and the ordinary post-processing prompt is separate from this profile-mode template after General has been initialized.

The visible variables in the mockup are `{{long_term_memory}}`, `{{short_term_memory}}`, `{{input_context}}`, and `{{transcript}}`. The production renderer must define bounded, labeled values for each, reject unknown or unclosed expressions, and never evaluate arbitrary helpers or template code. The mockup demonstrates source and editing behavior; it does not call an endpoint.

The native textarea supports pointer and keyboard selection, caret insertion, and tab navigation. Save and Cancel are buttons in document order; after Save, Cancel, or reset, focus returns to the prompt action button. The source label remains readable without color, the editor has a persistent label, and validation appears in an alert region. The detail pane scrolls at Handy's `680 × 570` starting window size so the prompt and routing controls remain reachable.
