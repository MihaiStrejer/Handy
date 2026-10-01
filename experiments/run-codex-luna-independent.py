"""Check whether Codex CLI reuses a shared prefix across independent sessions."""

import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


RESULTS = Path(__file__).resolve().parent / "results"
RESULTS.mkdir(exist_ok=True)
codex = shutil.which("codex")
if codex is None:
    raise RuntimeError("codex CLI is not on PATH")

reference = "The reference notes describe the project vocabulary and preferred wording for each speaker."
prefix = "Use the following stable reference as context. Do not use tools.\n\n" + " ".join([reference] * 995)
prompts = [
    ("first_a", prefix + "\n\nTranscript: so um send the revised report on Thursday. Output only the cleaned transcript."),
    ("different_b", prefix + "\n\nTranscript: well uh the meeting starts at noon on Friday. Output only the cleaned transcript."),
    ("same_a", prefix + "\n\nTranscript: so um send the revised report on Thursday. Output only the cleaned transcript."),
]
record = {"model": "gpt-6-luna", "cases": []}
for name, prompt in prompts:
    command = [
        codex, "exec", "--ephemeral", "--skip-git-repo-check", "--ignore-user-config",
        "-s", "read-only", "-m", "gpt-6-luna", "-c", 'model_reasoning_effort="none"', "--json", "-",
    ]
    start = time.perf_counter()
    result = subprocess.run(command, input=prompt, capture_output=True, text=True, cwd=tempfile.gettempdir(), timeout=120)
    elapsed = time.perf_counter() - start
    (RESULTS / f"codex-luna-independent-{name}.jsonl").write_text(result.stdout, encoding="utf-8")
    (RESULTS / f"codex-luna-independent-{name}.stderr.txt").write_text(result.stderr, encoding="utf-8")
    events = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    completed = next((event for event in events if event.get("type") == "turn.completed"), None)
    messages = [event["item"]["text"] for event in events if event.get("type") == "item.completed" and event.get("item", {}).get("type") == "agent_message"]
    case = {
        "name": name,
        "seconds": round(elapsed, 3),
        "exit_code": result.returncode,
        "usage": completed.get("usage") if completed else None,
        "output": messages[-1] if messages else None,
    }
    record["cases"].append(case)
    print(json.dumps(case), flush=True)
    if result.returncode != 0 or completed is None:
        break
(RESULTS / "measurements-codex-luna-independent.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
