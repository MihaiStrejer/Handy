"""Probe GPT-6 Luna through the installed Codex CLI using synthetic context."""

import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parent
RESULTS = ROOT / "results"
RESULTS.mkdir(exist_ok=True)
CODEX = shutil.which("codex")
if CODEX is None:
    raise RuntimeError("codex CLI is not on PATH")

REFERENCE = "The reference notes describe the project vocabulary and preferred wording for each speaker."
TWO_K = "so we reviewed the draft and uh updated the schedule for the team before sending the final report"
THREE_K = "well after the meeting we um updated the document and shared the next steps with everyone on the project"
PROMPTS = [
    (
        "warm_reference",
        "Keep the following reference material as context for later transcript cleanup. Do not use tools. Reply READY only.\n\n"
        + " ".join([REFERENCE] * 995),
    ),
    (
        "new_2k_transcript",
        "Clean this dictated transcript. Remove filler and repeated words, preserve meaning, and output only the full cleaned transcript. Do not summarize and do not use tools.\n\n"
        + " ".join([TWO_K] * 125),
    ),
    (
        "new_3k_transcript",
        "Clean this different dictated transcript. Remove filler and repeated words, preserve meaning, and output only the full cleaned transcript. Do not summarize and do not use tools.\n\n"
        + " ".join([THREE_K] * 185),
    ),
]


def run_case(name, prompt, thread_id):
    common = [
        "--skip-git-repo-check", "--ignore-user-config", "-m", "gpt-6-luna",
        "-c", 'model_reasoning_effort="none"', "--json",
    ]
    if thread_id is None:
        command = [CODEX, "exec", *common, "-s", "read-only", "-"]
    else:
        command = [CODEX, "exec", "resume", *common, thread_id, "-"]
    started = time.perf_counter()
    result = subprocess.run(command, input=prompt, capture_output=True, text=True, cwd=tempfile.gettempdir(), timeout=600)
    elapsed = time.perf_counter() - started
    (RESULTS / f"codex-luna-{name}.jsonl").write_text(result.stdout, encoding="utf-8")
    (RESULTS / f"codex-luna-{name}.stderr.txt").write_text(result.stderr, encoding="utf-8")
    events = []
    for line in result.stdout.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    started_event = next((event for event in events if event.get("type") == "thread.started"), None)
    completed_event = next((event for event in events if event.get("type") == "turn.completed"), None)
    messages = [event["item"]["text"] for event in events if event.get("type") == "item.completed" and event.get("item", {}).get("type") == "agent_message"]
    case = {
        "name": name,
        "seconds": round(elapsed, 3),
        "prompt_words": len(prompt.split()),
        "exit_code": result.returncode,
        "thread_id": started_event.get("thread_id") if started_event else thread_id,
        "usage": completed_event.get("usage") if completed_event else None,
        "output_words": len(messages[-1].split()) if messages else 0,
        "output_excerpt": messages[-1][:200] if messages else None,
    }
    print(json.dumps(case), flush=True)
    if result.returncode != 0 or completed_event is None:
        raise RuntimeError(f"{name} failed; see results/codex-luna-{name}.stderr.txt and .jsonl")
    return case


record = {"command": "codex exec / codex exec resume", "model": "gpt-6-luna", "cases": []}
thread = None
try:
    for name, prompt in PROMPTS:
        case = run_case(name, prompt, thread)
        record["cases"].append(case)
        thread = case["thread_id"]
finally:
    (RESULTS / "measurements-codex-luna.json").write_text(json.dumps(record, indent=2), encoding="utf-8")
