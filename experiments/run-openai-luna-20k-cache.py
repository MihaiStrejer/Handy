"""Prewarm a 20k-token Luna prompt and measure a cached follow-up."""

import hashlib
import json
import os
import re
import time
import urllib.error
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parent
RESULTS = ROOT / "results"
RESULTS.mkdir(exist_ok=True)
ENV_FILE = ROOT / ".env"
SETTINGS_FILE = ROOT.parent / "src-tauri" / "src" / "settings.rs"
MODEL = "gpt-6-luna"
REFERENCE = "The reference notes describe the project vocabulary and preferred wording for each speaker."
TRANSCRIPT = "so um i need to like send the the report by uh friday no wait make that thursday"
TARGET_TOKENS = 20_000


def load_key():
    key = os.environ.get("OPENAI_KEY")
    if key:
        return key
    for line in ENV_FILE.read_text(encoding="utf-8-sig").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("export "):
            stripped = stripped[7:].strip()
        name, separator, value = stripped.partition("=")
        if separator and name.strip() == "OPENAI_KEY":
            value = value.strip()
            if len(value) >= 2 and value[0] == value[-1] and value[0] in {'"', "'"}:
                value = value[1:-1]
            if value:
                return value
    raise RuntimeError("OPENAI_KEY is missing or empty")


def handy_system_prompt():
    source = SETTINGS_FILE.read_text(encoding="utf-8")
    start = source.index("fn default_post_process_prompts()")
    section = source[start:source.index("fn default_transcribe_gpu_device()", start)]
    match = re.search(r'prompt:\s*("(?:\\.|[^"\\])*")\.to_string\(\)', section)
    if match is None:
        raise RuntimeError("Could not find Handy's default post-processing prompt")
    return json.loads(match.group(1)).replace("${output}", "").strip()


KEY = load_key()


def api(path, payload, timeout=120):
    request = urllib.request.Request(
        "https://api.openai.com" + path,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": f"Bearer {KEY}", "Content-Type": "application/json"},
        method="POST",
    )
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            result = json.load(response)
    except urllib.error.HTTPError as exc:
        try:
            detail = json.load(exc).get("error", {}).get("message", "")
        except (ValueError, AttributeError):
            detail = ""
        raise RuntimeError(f"API HTTP {exc.code}: {detail[:300]}") from None
    return result, round(time.perf_counter() - start, 3)


BASE_PROMPT = handy_system_prompt()


def developer_message(repetitions, breakpoint=True):
    content = {"type": "input_text", "text": BASE_PROMPT + "\n\nReference notes:\n" + " ".join([REFERENCE] * repetitions)}
    if breakpoint:
        content["prompt_cache_breakpoint"] = {"mode": "explicit"}
    return {"role": "developer", "content": [content]}


def count_tokens(repetitions):
    response, seconds = api(
        "/v1/responses/input_tokens",
        {"model": MODEL, "input": [developer_message(repetitions)]},
    )
    count = response["input_tokens"]
    print(f"Token count: repetitions={repetitions}, tokens={count}, seconds={seconds}", flush=True)
    return count


repetitions = 1400
count = count_tokens(repetitions)
for _ in range(4):
    if abs(count - TARGET_TOKENS) <= 20:
        break
    estimated = max(1, round(repetitions * TARGET_TOKENS / count))
    if estimated == repetitions:
        estimated += 1 if count < TARGET_TOKENS else -1
    repetitions = estimated
    count = count_tokens(repetitions)
if not 19_900 <= count <= 20_100:
    raise RuntimeError(f"Could not calibrate prompt near 20k tokens; got {count}")

stable = developer_message(repetitions)
common = {
    "model": MODEL,
    "reasoning": {"effort": "none"},
    "store": False,
    "prompt_cache_options": {"mode": "explicit", "ttl": "30m"},
}
record = {
    "model": MODEL,
    "developer_input_tokens_counted": count,
    "reference_repetitions": repetitions,
    "developer_prompt_sha256": hashlib.sha256(stable["content"][0]["text"].encode("utf-8")).hexdigest(),
    "cases": [],
}


def save():
    (RESULTS / "measurements-openai-luna-20k-cache.json").write_text(json.dumps(record, indent=2), encoding="utf-8")


def run_case(name, payload):
    response, seconds = api("/v1/responses", payload)
    output = "".join(
        part.get("text", "")
        for item in response.get("output", [])
        for part in item.get("content", [])
        if part.get("type") == "output_text"
    )
    case = {"name": name, "seconds": seconds, "status": response.get("status"), "usage": response.get("usage"), "output": output}
    record["cases"].append(case)
    save()
    print(json.dumps(case), flush=True)


run_case("prewarm", {**common, "input": [stable], "prompt_cache_options": {**common["prompt_cache_options"], "prewarm": True}})
run_case("cached_short_transcript", {
    **common,
    "input": [stable, {"role": "user", "content": [{"type": "input_text", "text": TRANSCRIPT}]}],
    "max_output_tokens": 128,
})
run_case("uncached_short_transcript", {
    **common,
    "input": [developer_message(repetitions, breakpoint=False), {"role": "user", "content": [{"type": "input_text", "text": TRANSCRIPT}]}],
    "max_output_tokens": 128,
})
