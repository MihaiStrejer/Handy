# History estimate sources

Verified 2026-09-27 against official documentation. These notes support implementation of optional USD estimates; the design board's fictional rates are not production data.

## GPT-6 Luna

Direct OpenAI API, exact model `gpt-6-luna`, USD per million text tokens:

| Context | Ordinary input | Cached input | Cache writes | Output |
| --- | --- | --- | --- | --- |
| At most 272,000 input tokens | 0.10 | 0.01 | 0.125 | 0.50 |
| More than 272,000 input tokens | 0.20 | 0.02 | 0.25 | 0.75 |

These are Standard rates. The context threshold applies to the whole request. The model page also documents Flex at half the Standard price and Fast mode at twice the applicable price. Regional pricing adds a premium; do not apply the direct global endpoint's rate to other endpoints. [GPT-6 Luna model documentation](https://developers.openai.com/api/docs/models/gpt-6-luna), [API pricing](https://developers.openai.com/api/docs/pricing).

For initial support, use a verified effective service tier and the exact direct endpoint. Unrecognized model aliases, missing pricing categories, unverified tier and proxy endpoints should yield an unavailable estimate. Persist the chosen rate snapshot and verification date with each estimate.

## Usage fields and arithmetic

Chat Completions exposes `prompt_tokens`, `completion_tokens`, `total_tokens`, `prompt_tokens_details.cached_tokens`, `prompt_tokens_details.cache_write_tokens`, and `completion_tokens_details.reasoning_tokens`. Usage breakdowns are optional. The returned `service_tier` reflects the effective processing tier, which can differ from the request. [Chat Completions reference](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create).

For GPT-6, ordinary input excludes cached reads and cache writes. Charge each input token in its reported category: `(input - cached - writes) * input_rate + cached * cached_rate + writes * write_rate`; add output cost, then divide by one million. Cache-write pricing replaces the ordinary input rate for those tokens; it is not an additional fee. Missing category counts must not silently become zero. Reject inconsistent negative remainders and preserve the original usage as incomplete evidence. [Prompt caching guide](https://developers.openai.com/api/docs/guides/prompt-caching).

The History spec's simple formula remains valid for adapters without separate cache-write billing. The Luna adapter requires the additional category above. Keep computations exact until formatting and do not add reasoning tokens again when they are already included in the reported output total.
