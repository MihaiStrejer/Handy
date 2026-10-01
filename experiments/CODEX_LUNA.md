# Codex CLI and GPT-6 Luna cache probe

The installed `codex` CLI 0.157.1 accepted `gpt-6-luna` with the existing ChatGPT login. It reported cached input tokens when a saved Codex session was resumed and in one independent session with a shared prompt prefix. This proves that the harness can benefit from OpenAI prompt caching. The CLI did not provide a reliable cache hit across every independent request in this small test.

## Reproduce

From the repository root, with `codex login status` reporting a logged-in account, run:

```powershell
uv run python experiments/run-codex-luna.py
uv run python experiments/run-codex-luna-diverse.py
uv run python experiments/run-codex-luna-independent.py
uv run python experiments/run-codex-luna-ten.py
```

The first script creates a saved Codex session and resumes it twice. The second script resumes that same session, so run it after the first. The third script creates three separate ephemeral sessions. All requests select `gpt-6-luna`, set `model_reasoning_effort="none"`, use synthetic text, and ask the agent to avoid tools. The scripts store raw JSONL events, stderr, generated output, and measurement summaries in ignored `experiments/results/` files. They use the CLI's ChatGPT login; they do not call the OpenAI API with a project API key.

The fourth script runs ten independent ephemeral requests. Each repeats the same roughly 14k-token synthetic reference, then adds a different seeded transcript with 60 numbered items and approximately 3k dynamic input tokens. It asks Luna to clean and return every item. The exact full request has about 33k input tokens as reported by Codex, including its own instructions. There is no separate warm-up excluded from the ten results.

## Measurements

| Request | Wall time | Reported cached input | Output |
| --- | ---: | ---: | --- |
| Load roughly 14k tokens of synthetic reference into a saved session | 3.846 s | 0 | `READY` |
| Resume with a repeated 2k-token transcript | 3.504 s | 29,440 cumulative | 17 words |
| Resume with a different repeated 3k-token transcript | 3.694 s | 60,928 cumulative | 18 words |
| Resume with 80 distinct numbered items | 79.884 s | 96,512 cumulative | 3,040 words; all 80 items present |

`codex exec --json` reports cumulative usage for the saved session. The increments in cached input were 29,440, 31,488, and 35,584 tokens across the three resumed turns. The totals include Codex's own instructions and conversation history, so they do not isolate the synthetic reference's cache hit. The short 2k and 3k cases repeated one sentence many times, which Luna condensed to one sentence. Their 3.5–3.7-second timings do not measure full-length output. The 80-item case produced 3,897 additional output tokens and took 79.9 seconds; prompt caching did not remove output-generation time.

In three separate ephemeral sessions, the first request reported 0 cached tokens, the second request with the same synthetic prefix but a changed transcript reported 13,056 cached tokens, and a third request identical to the first reported 0 cached tokens. Their wall times were 3.648, 3.825, and 3.693 seconds, respectively. A matching prefix can be reused across Codex sessions, but this run does not establish a reliable hit rate or a latency benefit for full transcripts.

### Ten full-output requests

| Run | Wall time | Cached input tokens | Output tokens |
| ---: | ---: | ---: | ---: |
| 1 | 53.920 s | 13,056 | 2,798 |
| 2 | 53.724 s | 0 | 2,791 |
| 3 | 54.293 s | 0 | 2,800 |
| 4 | 54.820 s | 0 | 2,787 |
| 5 | 54.163 s | 7,936 | 2,794 |
| 6 | 54.268 s | 0 | 2,793 |
| 7 | 54.462 s | 13,056 | 2,786 |
| 8 | 53.360 s | 13,056 | 2,792 |
| 9 | 103.883 s | 0 | 2,790 |
| 10 | 54.806 s | 11,008 | 2,847 |

The arithmetic mean is 59.170 seconds, the median is 54.281 seconds, and nine of ten requests finished between 53.360 and 54.820 seconds. Run 9 took 103.883 seconds without an error or retry reported in its saved events or stderr, so its cause is unknown. All ten outputs contained all 60 separately numbered items; each output had 2,220 words and about 2,798 tokens on average. Five requests reported some cached input. Cache hits did not clearly reduce total time for this full-output workload. The raw measurements are in ignored `experiments/results/measurements-codex-luna-ten.json` and the per-run JSONL and output files.

## Cache controls and Handy fit

[OpenAI's prompt-caching guide](https://developers.openai.com/api/docs/guides/prompt-caching) says Agents API calls using the managed Codex harness follow the same prompt-caching behavior as Responses API calls, while a continuing session alone does not guarantee a cache hit. GPT-6 models can place an explicit cache breakpoint after stable developer content, keep changing text after that breakpoint, and request a 30-minute minimum cache lifetime through the Responses API. The [published Codex configuration schema](https://developers.openai.com/codex/config-schema.json) and this CLI's help do not expose `prompt_cache_options` or `prompt_cache_breakpoint` as CLI configuration. The CLI experiment therefore demonstrates automatic reuse but does not demonstrate control over a dictionary-specific breakpoint or cache lifetime.

The Codex CLI accepts a user prompt and manages its own instructions and history. These probes did not send the synthetic reference as a separate developer or system message. A Handy integration that needs an exact, independently managed system prompt, dictionary, and dynamic transcript should evaluate the [Responses API's explicit breakpoint](https://developers.openai.com/api/docs/guides/prompt-caching) or the [Agents API's managed Codex harness](https://developers.openai.com/api/docs/guides/agents). The current CLI results alone are insufficient to promise a response under five seconds when the output must preserve a long transcript.
