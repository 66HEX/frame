#!/usr/bin/env python3
"""Index Frame's reusable UI builders and all their source call sites. No dependencies."""
import json
import re
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
dest = root / 'docs/frame-app/design-system'
paths = subprocess.check_output(['rg', '--files', 'frame-app/src/app'], cwd=root, text=True).splitlines()
sources = {path: (root / path).read_text() for path in paths if path.endswith('.rs') and 'test' not in path}
builders = []
for path, text in sources.items():
    for match in re.finditer(r'\bfn\s+(\w+)\s*(?:<[^\n]*>)?\s*\([\s\S]*?\)\s*->\s*(?:gpui::)?(Div|Stateful<(?:gpui::)?Div>|impl IntoElement)\s*\{', text):
        name = match[1]
        if name == 'render':
            continue
        builders.append({'name': name, 'path': path, 'line': text.count('\n', 0, match.start()) + 1})
lookup = {entry['name']: entry for entry in builders}
sites = []
for path, text in sources.items():
    for match in re.finditer(r'\b(' + '|'.join(map(re.escape, sorted(lookup))) + r')\s*\(', text):
        if re.search(r'\bfn\s*$', text[max(0, match.start()-8):match.start()]):
            continue
        sites.append({'builder': match[1], 'path': path, 'line': text.count('\n', 0, match.start()) + 1})
ids = []
for path, text in sources.items():
    for match in re.finditer(r'\.id\(\s*(?:format!\(\s*)?"([^"\n]+)"', text):
        ids.append({'id_or_pattern': match[1], 'path': path, 'line': text.count('\n', 0, match.start()) + 1})
input_source = sources['frame-app/src/app/input/runtime.rs']
inputs = [{'kind': kind, 'label': label} for kind, label in re.findall(r'Self::(\w+) => "([^"]+)"', input_source)]
enums = []
for path, text in sources.items():
    for match in re.finditer(r'enum ((?:\w*RangeTarget)|FrameTextInputKind)\s*\{([^}]+)\}', text):
        enums.append({'name': match[1], 'variants': re.findall(r'^\s*(\w+)\s*,', match[2], re.M), 'path': path})
report = {'scope': 'Rust UI builders and call sites, excluding tests. Dynamic lists are indexed as builders/patterns, not a fixed runtime count.', 'text_inputs': inputs, 'dynamic_targets': enums, 'builders': sorted(builders, key=lambda e: (e['path'], e['line'])), 'call_sites': sites, 'explicit_ids': ids}
dest.mkdir(parents=True, exist_ok=True)
(dest/'inventory.json').write_text(json.dumps(report, indent=2, ensure_ascii=False)+'\n')
lines = ['# Inwentaryzacja UI Frame', '', 'Wygenerowano z kodu przez `python3 script/inventory-ui.py`. Zestawienie obejmuje konstruktory elementów i każde miejsce ich użycia poza testami. Listy plików, kodeków, presetów, fontów i ścieżek są dynamiczne; nie mają stałej liczby instancji.', '', f'Rodzaje pól tekstowych: {len(inputs)}; konstruktory UI: {len(builders)}; miejsca użycia: {len(sites)}; jawne identyfikatory lub ich wzorce: {len(ids)}. Pełny indeks z numerami linii jest w [inventory.json](inventory.json).', '', '## Pola tekstowe', '', '| Rodzaj | Etykieta dostępności |', '| --- | --- |']
lines += [f"| `{entry['kind']}` | {entry['label']} |" for entry in inputs]
lines += ['', '## Parametry suwaków', '']
for entry in enums:
    if entry['name'] == 'FrameTextInputKind': continue
    lines += [f"- `{entry['name']}`: " + ', '.join(entry['variants']) + '.']
lines += ['', 'Dodatkowo: pozycja odtwarzania, początek i koniec przycięcia, przezroczystość nakładki, hue i saturation/value w pickerze kolorów.', '', '## Komponenty i miejsca użycia', '', '| Konstruktor | Źródło | Liczba wywołań |', '| --- | --- | --- |']
for entry in report['builders']:
    short=entry['path'].removeprefix('frame-app/src/app/')
    lines.append(f"| `{entry['name']}` | [{short}](../../../frame-app/src/app/{short}#L{entry['line']}) | {sum(site['builder']==entry['name'] for site in sites)} |")
lines += ['', '## Jawne identyfikatory elementów', '', 'Identyfikatory obejmują również kontenery, które mają znaczenie dla testów geometrii i dostępności. Pełne argumenty dynamicznych kontrolek są w podanych konstruktorach.', '', '| Identyfikator lub wzorzec | Źródło |', '| --- | --- |']
for entry in ids:
    short=entry['path'].removeprefix('frame-app/src/app/')
    lines.append(f"| `{entry['id_or_pattern']}` | [{short}](../../../frame-app/src/app/{short}#L{entry['line']}) |")
(dest/'INVENTORY.md').write_text('\n'.join(lines)+'\n')
print(f'{len(inputs)} inputs; {len(builders)} builders; {len(sites)} call sites; {len(ids)} IDs')
