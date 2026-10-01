from pathlib import Path
from base64 import b64encode
from hashlib import sha256
from html import escape
import json
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
capture = json.loads((ROOT / 'design/proof/architecture/capture.json').read_text(encoding='utf-8'))
page = (HERE / 'template.html').read_text(encoding='utf-8')
for name in ('dictionary-annotated', 'context-original', 'overlay'):
    data = (ROOT / f'design/proof/architecture/{name}.png').read_bytes()
    page = page.replace(f'@@{name}@@', 'data:image/png;base64,' + b64encode(data).decode())
sources = [
    ('S1', 'Startup prepares audio, recognition, history and profile state', 'src-tauri/src/lib.rs', 191, 232),
    ('S2', 'Invocation captures the original destination and starts context work', 'src-tauri/src/actions.rs', 535, 578),
    ('S3', 'Background capture resolves a profile and publishes display metadata', 'src-tauri/src/context_profiles/mod.rs', 19, 90),
    ('S4', 'Profile snapshot, persistent catalog and process memory', 'src-tauri/src/context_profiles/storage.rs', 253, 449),
    ('S5', 'Routing and prompt inheritance', 'src-tauri/src/context_profiles/routing.rs', 13, 116),
    ('S6', 'Windows selection capture and explicit availability', 'src-tauri/src/context_profiles/capture.rs', 140, 186),
    ('S7', 'Request assembly, compatibility check, response validation and field recheck', 'src-tauri/src/context_profiles/request.rs', 39, 232),
    ('S8', 'Message roles and HTTP request', 'src-tauri/src/llm_client.rs', 338, 395),
    ('S9', 'Session identity, prediction authority and cleanup', 'src-tauri/src/context_profiles/session.rs', 92, 309),
    ('S10', 'Processing errors suppress output', 'src-tauri/src/actions.rs', 499, 533),
    ('S11', 'History, cancellation, final focus check and output dispatch', 'src-tauri/src/actions.rs', 898, 971),
    ('S12', 'HTTP response model contains content, without cached usage', 'src-tauri/src/llm_client.rs', 112, 137),
]
evidence = []
for sid, title, path, start, end in sources:
    data = (ROOT / path).read_bytes()
    lines = data.decode('utf-8').splitlines()
    excerpt = '\n'.join(f'{i}: {line}' for i, line in enumerate(lines[start - 1:end], start))
    evidence.append(f'<details id="{sid}"><summary>{sid} · {escape(title)}<small>{path}:{start}–{end}</small></summary><p class="caption">Working-tree source · SHA-256 {sha256(data).hexdigest()}</p><pre>{escape(excerpt)}</pre><a href="../../../../{path}">Open source file</a></details>')
page = page.replace('@@EVIDENCE@@', '\n'.join(evidence))
page = page.replace('@@CAPTURE@@', escape(capture['captured_at']))
page = page.replace('@@COMMIT@@', subprocess.check_output(['git', 'rev-parse', '--short', 'HEAD'], cwd=ROOT, text=True).strip())
assert '@@' not in page
(HERE / 'index.html').write_text(page, encoding='utf-8')
print('Built H with three embedded captures and twelve source excerpts.')
