"""Time ten independent Luna cleanup requests with shared reference text."""

import json
import random
import shutil
import statistics
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
PREFIX = (
    "Use the following stable reference when cleaning the dictated transcript. Do not use tools.\n\n"
    + " ".join([REFERENCE] * 995)
    + "\n\nTranscript:\n"
)
NAMES = ["Mara", "Jules", "Anika", "Sorin", "Leah", "Mateo", "Nina", "Owen", "Priya", "Theo"]
OBJECTS = ["route map", "release notes", "training schedule", "budget sheet", "design draft", "support checklist", "client summary", "meeting plan"]
DAYS = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]


def transcript_for(seed):
    rng = random.Random(seed)
    items = []
    for number in range(1, 61):
        name = rng.choice(NAMES)
        reviewer = rng.choice([candidate for candidate in NAMES if candidate != name])
        obj = rng.choice(OBJECTS)
        day = rng.choice(DAYS)
        deadline = rng.choice([candidate for candidate in DAYS if candidate != day])
        amount = rng.randrange(100, 9500)
        team = rng.randrange(1, 18)
        items.append(
            f"item {number}: so um {name} checked version {number} of the {obj} on {day}, "
            f"and {reviewer} will send the revised notes to team {team} before {deadline}; "
            f"the amount for this item is {amount} dollars, and uh the owner confirmed the update."
        )
    return "\n".join(items)


def write_record(record):
    (RESULTS / "measurements-codex-luna-ten.json").write_text(json.dumps(record, indent=2), encoding="utf-8")


record = {"model": "gpt-6-luna", "runs": [], "shared_reference_repetitions": 995, "item_count_per_run": 60}
for index in range(1, 11):
    name = f"{index:02d}"
    transcript = transcript_for(20260926 + index)
    prompt = PREFIX + transcript + "\n\nClean this transcript. Remove filler words and repair punctuation. Preserve all 60 numbered items, their order, names, numbers, dates, and amounts. Output every cleaned item with its number. Do not summarize or use tools."
    command = [
        CODEX, "exec", "--ephemeral", "--skip-git-repo-check", "--ignore-user-config",
        "-s", "read-only", "-m", "gpt-6-luna", "-c", 'model_reasoning_effort="none"', "--json", "-",
    ]
    print(f"Starting run {index}/10; transcript_words={len(transcript.split())}", flush=True)
    started = time.perf_counter()
    try:
        result = subprocess.run(command, input=prompt, capture_output=True, text=True, cwd=tempfile.gettempdir(), timeout=240)
    except subprocess.TimeoutExpired as exc:
        run = {"run": index, "seconds": 240, "error": repr(exc)}
        record["runs"].append(run)
        write_record(record)
        raise
    elapsed = time.perf_counter() - started
    (RESULTS / f"codex-luna-ten-{name}.jsonl").write_text(result.stdout, encoding="utf-8")
    (RESULTS / f"codex-luna-ten-{name}.stderr.txt").write_text(result.stderr, encoding="utf-8")
    events = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    completed = next((event for event in events if event.get("type") == "turn.completed"), None)
    messages = [event["item"]["text"] for event in events if event.get("type") == "item.completed" and event.get("item", {}).get("type") == "agent_message"]
    output = messages[-1] if messages else ""
    (RESULTS / f"codex-luna-ten-{name}-output.txt").write_text(output, encoding="utf-8")
    run = {
        "run": index,
        "seconds": round(elapsed, 3),
        "exit_code": result.returncode,
        "transcript_words": len(transcript.split()),
        "output_words": len(output.split()),
        "numbered_items_in_output": sum(f"Item {number}:" in output or f"{number}." in output for number in range(1, 61)),
        "usage": completed.get("usage") if completed else None,
    }
    record["runs"].append(run)
    write_record(record)
    print(json.dumps(run), flush=True)
    if result.returncode != 0 or completed is None:
        raise RuntimeError(f"Codex request {index} failed; see results/codex-luna-ten-{name}.stderr.txt")

seconds = [run["seconds"] for run in record["runs"]]
record["summary"] = {
    "mean_seconds": round(statistics.mean(seconds), 3),
    "median_seconds": round(statistics.median(seconds), 3),
    "min_seconds": min(seconds),
    "max_seconds": max(seconds),
    "all_items_present_runs": sum(run["numbered_items_in_output"] == 60 for run in record["runs"]),
    "cache_hit_runs": sum(run["usage"]["cached_input_tokens"] > 0 for run in record["runs"]),
}
write_record(record)
print(json.dumps(record["summary"]), flush=True)
