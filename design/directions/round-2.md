# History inspector: selected layout and failure states

Status: Option B selected by the user. This refinement moves the post-processing status immediately after the timestamp in the entry header. It places the failure reason inside the same details section that normally shows provider, profile, duration, tokens and estimated cost. [Open the interactive samples](round-2.html).

The History sidebar, audio controls and action icons retain their existing placement. On a wide window, the inspector stays beside the list; in a compact window it fills the History content pane. Closing it returns to the selected entry. The full date remains available through the timestamp's accessible title; the compact header reads `Sep 27 · 12:34 PM` followed by `Post-processed` or `Post-process failed`. If space is insufficient, the header wraps without overlapping the action icons.

## Entry details section

A successful run shows its normal provider/profile/time/token/cost summary. A failed run keeps the known provider, profile and elapsed time, then displays the concrete reason below those values, within the same section. Unavailable token/cost figures are omitted from this compact summary and explained in the inspector. Usage that was actually returned is retained even when response validation fails. The failure is visible with the inspector closed and is not communicated by color alone.

## Failure examples

| Sample | Entry reason | Inspector explanation and recovery |
| --- | --- | --- |
| Rate limit | The provider rate limit was reached. | HTTP 429 with a normalized rate-limit code. Wait before trying again; check provider limits if it persists. |
| API key rejected | The provider rejected the API key. | HTTP 401. Check the key in Post Process settings. Never display the key. |
| Timeout | No complete response arrived within 60 seconds. | Client deadline expired. Check connectivity/provider status; no claim that cancellation prevented billing. |
| Invalid response | The response did not contain a valid rewrite. | HTTP 200 followed by response validation failure. Preserve reported usage/cost. Check model support for the profile format. |
| Missing model | Select a post-processing model to run this profile. | Local configuration failure. No request was sent and there is no provider charge for this run. |

The inspector places the error title, plain-language reason, next step and original-transcript/output outcome above its metric rows. Calls identifies the stage and HTTP status when available. Requests shows the body that was actually dispatched; the missing-model example instead says No request was sent. The sample recovery copy does not introduce a replay button: replaying a frozen profile context remains outside the current scope.

The provider status alone is insufficient to diagnose every error: 429 can represent different provider conditions, and 400/403 do not justify a guessed explanation. Implementation must classify a bounded allowlist of provider error codes and use a generic status-based message when the cause is unknown. Provider error prose is not shown or retained verbatim because it may echo credentials or context.

## Artifacts and verification

The interactive board includes success, five failure cases, an older entry and a gallery of failure cards that open the matching inspector. All data is synthetic. Static screenshots are in `proof/round-2/`. `node scripts/verify-history-refinement.mjs` checks status placement, error location, all five cases, request/call distinctions, closing and restoring focus, gallery selection, compact dark/light layouts and browser errors. `uv run python scripts/build-history-refinement.py` regenerates the self-contained board using round one's local assets. These are design artifacts; the native app is unchanged.
