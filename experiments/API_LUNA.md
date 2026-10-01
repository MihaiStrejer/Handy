# One direct GPT-6 Luna API probe

The direct Responses API request took 3.438 seconds end to end for one short Handy post-processing example. It used Handy's built-in "Improve Transcriptions" prompt from `src-tauri/src/settings.rs`, applying the same `${output}` removal as `build_system_prompt` in `src-tauri/src/actions.rs`. The prompt was sent as a system message and the transcript as a user message. The request selected `gpt-6-luna`, set reasoning effort to `none`, capped output at 128 tokens, and set `store` to `false`.

| Input | Result |
| --- | --- |
| Transcript | `so um i need to like send the the report by uh friday no wait make that thursday` |
| Output | `So I need to send the report by Friday. No, wait, make that Thursday.` |
| Full request time | 3.438 seconds |
| Token usage | 247 input; 22 output; 0 cached |

The system prompt was 909 characters. At 247 total input tokens, this request was below [OpenAI's 1,024-token minimum for prompt caching on GPT-5.6 and later](https://developers.openai.com/api/docs/guides/prompt-caching). This one short request does not measure cached-prefix behavior or full-transcript output time. The [Codex CLI comparison](CODEX_LUNA.md) used larger synthetic inputs and is not a like-for-like latency comparison.

Run the probe from the repository root with `OPENAI_KEY` in ignored `experiments/.env`:

```powershell
uv run python experiments/run-openai-luna-one.py
```

The script reads the key without printing it and writes only prompt metadata, usage, output, and timing to ignored `experiments/results/measurements-openai-luna-one.json`.

## 20k-token prefix cache probe

`run-openai-luna-20k-cache.py` appended 1,412 copies of a synthetic reference sentence to Handy's default prompt and used OpenAI's input-token count endpoint to set the stable developer message to 19,995 tokens. It placed an explicit breakpoint after that message and requested a 30-minute cache lifetime. The prewarm request generated no output. A follow-up sent the same developer message and the short transcript above. A control request sent the same 20,018-token input with explicit-only caching and no breakpoint, which yielded zero cached tokens.

| Request | Wall time | Input tokens | Cache write tokens | Cached tokens | Output tokens |
| --- | ---: | ---: | ---: | ---: | ---: |
| Prewarm stable prompt | 1.592 s | 19,995 | 19,992 | 0 | 0 |
| Short transcript, cached | 1.289 s | 20,018 | 0 | 19,992 | 22 |
| Same transcript, caching disabled | 1.715 s | 20,018 | 0 | 0 | 22 |

The API reported reuse of nearly the entire 20k-token stable prefix. Both transcript requests produced the same output shown above. The one cached request was 0.426 seconds faster than the control, but these single timings are not a stable latency estimate. The reference sentence was synthetic, and the generated output was only 22 tokens; this probe does not measure a large dictionary's usefulness or the time to return a full long transcript. The [OpenAI prompt-caching guide](https://developers.openai.com/api/docs/guides/prompt-caching) documents explicit breakpoints and prewarming.

Run it with the same ignored `experiments/.env` file:

```powershell
uv run python experiments/run-openai-luna-20k-cache.py
```

The script stores counts, usage, timing, and output without the key or prompt text in ignored `experiments/results/measurements-openai-luna-20k-cache.json`.

### Standard API cost

As of 2026-09-26, [OpenAI lists GPT-6 Luna](https://developers.openai.com/api/docs/models/gpt-6-luna) at $0.10 per million ordinary input tokens, $0.01 per million cached input tokens, $0.125 per million cache-write tokens, and $0.50 per million output tokens for this context size. Applying those rates to the API-reported usage gives approximately $0.002499 for the prewarm, $0.000214 for the cached 22-token response, and $0.002013 for the uncached control. These are calculated model charges, not a billing statement.

For a hypothetical later request with 20k cached input tokens, 3k new input tokens, and 2,800 output tokens, the same rates give about $0.0019 per request after the prewarm. Without a cache hit, that request would be about $0.0037. The long-output latency and output quality for this direct API path remain unmeasured.
