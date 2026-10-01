from pathlib import Path
from html import escape
import hashlib
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sources = [('S1','src-tauri/src/actions.rs',499,565),('S2','src-tauri/src/context_profiles/mod.rs',19,100),('S3','src-tauri/src/context_profiles/request.rs',47,116),('S4','src-tauri/src/context_profiles/request.rs',211,292),('S5','src-tauri/src/actions.rs',882,965),('S6','src-tauri/src/context_profiles/session.rs',269,343),('S7','src-tauri/src/context_profiles/capture.rs',161,249),('S8','plan.md',149,163)]
evidence = []
sources += [('S9','src-tauri/src/context_profiles/routing.rs',40,120),('S10','src-tauri/src/context_profiles/template.rs',1,45),('S11','src-tauri/src/lib.rs',211,236),('S12','src-tauri/src/context_profiles/request.rs',119,170)]
for key, name, start, end in sources:
    raw = (ROOT/name).read_bytes()
    lines = raw.decode('utf-8').splitlines()
    excerpt = '\n'.join(f'{n}: {line}' for n,line in enumerate(lines,start=1) if start <= n <= end)
    evidence.append(f'<details id="{key}"><summary>{key} · {escape(name)}:{start}</summary><p class="caption">Working tree SHA-256: {hashlib.sha256(raw).hexdigest()}</p><pre>{escape(excerpt)}</pre></details>')
html = (HERE/'page.template.html').read_text(encoding='utf-8')
commit = subprocess.check_output(['git','rev-parse','--short','HEAD'],cwd=ROOT,text=True).strip()
(HERE/'index.html').write_text(html.replace('@@SOURCES@@','\n'.join(evidence)).replace('@@COMMIT@@',commit),encoding='utf-8')
print(HERE/'index.html')
