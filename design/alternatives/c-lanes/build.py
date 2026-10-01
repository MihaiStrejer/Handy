from pathlib import Path
from html import escape
import subprocess
ROOT=Path(__file__).resolve().parents[3]
OUT=Path(__file__).parent
sources=[('S1','Startup','src-tauri/src/lib.rs',217,225),('S2','Invocation','src-tauri/src/actions.rs',539,573),('S3','Parallel capture and widget','src-tauri/src/context_profiles/mod.rs',19,79),('S4','Roles and parser','src-tauri/src/context_profiles/request.rs',48,119),('S5','Request and input checks','src-tauri/src/context_profiles/request.rs',188,277),('S6','History and output','src-tauri/src/actions.rs',901,966),('S7','Configuration snapshot','src-tauri/src/context_profiles/storage.rs',263,286),('S8','Memory API','src-tauri/src/context_profiles/storage.rs',335,381),('S9','Prediction and cleanup','src-tauri/src/context_profiles/session.rs',269,312),('S10','Processing-time settings','src-tauri/src/actions.rs',499,532),('S11','Compatibility cache','src-tauri/src/context_profiles/request.rs',122,187),('S12','Native Edit boundary','src-tauri/src/context_profiles/capture.rs',147,174),('S13','Routing','src-tauri/src/context_profiles/routing.rs',43,130)]
evidence=[]
for sid,title,path,first,last in sources:
    lines=(ROOT/path).read_text(encoding='utf-8').splitlines()
    code='\n'.join(f'{i+1:4}  {line}' for i,line in enumerate(lines) if first<=i+1<=last)
    evidence.append(f'<details id="{sid}"><summary>{sid} · {title}<small>{path}:{first}</small></summary><pre>{escape(code)}</pre></details>')
page=(OUT/'template.html').read_text(encoding='utf-8')
commit=subprocess.check_output(['git','rev-parse','--short','HEAD'],cwd=ROOT,text=True).strip()
(OUT/'index.html').write_text(page.replace('@@COMMIT@@',escape(commit)).replace('@@EVIDENCE@@','\n'.join(evidence)),encoding='utf-8')
print(OUT/'index.html')
