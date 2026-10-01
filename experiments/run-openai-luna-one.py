"""Send one short Handy post-processing request to GPT-6 Luna via Responses API."""

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
TRANSCRIPT = "so um i need to like send the the report by uh friday no wait make that thursday"


def load_key():
    key = os.environ.get("OPENAI_KEY")
    if key:
        return key
    if not ENV_FILE.is_file():
        raise RuntimeError("experiments/.env is missing")
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
            if not value:
                raise RuntimeError("OPENAI_KEY is empty")
            return value
    raise RuntimeError("OPENAI_KEY is missing from experiments/.env")


def handy_system_prompt():
    source = SETTINGS_FILE.read_text(encoding="utf-8")
    start = source.index("fn default_post_process_prompts()")
    section = source[start:source.index("fn default_transcribe_gpu_device()", start)]
    match = re.search(r'prompt:\s*("(?:\\.|[^"\\])*")\.to_string\(\)', section)
    if match is None:
        raise RuntimeError("Could not find Handy's default post-processing prompt")
    template = json.loads(match.group(1))
    return template.replace("${output}", "").strip()


prompt = handy_system_prompt()
payload = {
    "model": "gpt-6-luna",
    "reasoning": {"effort": "none"},
    "input": [
        {"role": "system", "content": [{"type": "input_text", "text": prompt}]},
        {"role": "user", "content": [{"type": "input_text", "text": TRANSCRIPT}]},
    ],
    "max_output_tokens": 128,
    "store": False,
}
body = json.dumps(payload).encode("utf-8")
request = urllib.request.Request(
    "https://api.openai.com/v1/responses",
    data=body,
    headers={"Authorization": f"Bearer {load_key()}", "Content-Type": "application/json"},
    method="POST",
)
started = time.perf_counter()
try:
    with urllib.request.urlopen(request, timeout=90) as response:
        result = json.load(response)
except urllib.error.HTTPError as exc:
    print(f"OpenAI API request failed with HTTP {exc.code}; response body omitted", flush=True)
    raise SystemExit(1) from None
elapsed = time.perf_counter() - started
output = "".join(
    part.get("text", "")
    for item in result.get("output", [])
    for part in item.get("content", [])
    if part.get("type") == "output_text"
)
record = {
    "model": result.get("model"),
    "status": result.get("status"),
    "seconds": round(elapsed, 3),
    "system_prompt_sha256": hashlib.sha256(prompt.encode("utf-8")).hexdigest(),
    "system_prompt_characters": len(prompt),
    "transcript_words": len(TRANSCRIPT.split()),
    "usage": result.get("usage"),
    "output": output,
}
(RESULTS / "measurements-openai-luna-one.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
print(json.dumps(record), flush=True)
