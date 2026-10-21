#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

K="${KICAD_CLI:-/mnt/c/Users/$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r')/AppData/Local/Programs/KiCad/10.0/bin/kicad-cli.exe}"
OUT=out
mkdir -p "$OUT"
win() { if command -v wslpath >/dev/null; then wslpath -w "$1"; else echo "$1"; fi; }

"$K" sch erc --format json --exit-code-violations -o "$(win "$OUT/erc.json")" turret-pdb.kicad_sch >/dev/null || true
"$K" pcb drc --format json --schematic-parity --refill-zones --save-board \
    -o "$(win "$OUT/drc.json")" turret-pdb.kicad_pcb >/dev/null || true

python3 - "$OUT" <<'PY'
import json, sys
from collections import Counter
out = sys.argv[1]
erc = [v for s in json.load(open(f"{out}/erc.json"))["sheets"] for v in s["violations"]]
drc = json.load(open(f"{out}/drc.json"))
errors = [v for v in drc["violations"] if v["severity"] == "error"]
print(f"ERC {len(erc)}, DRC errors {len(errors)}, warnings {len(drc['violations']) - len(errors)}, "
      f"unconnected {len(drc['unconnected_items'])}, schematic parity {len(drc['schematic_parity'])}")
for v in erc + errors + drc["unconnected_items"] + drc["schematic_parity"]:
    print("  -", v["type"], "|", v["description"], "|", " / ".join(i["description"] for i in v["items"]))
sys.exit(1 if erc or errors or drc["unconnected_items"] or drc["schematic_parity"] else 0)
PY

PY="${PLATES_PYTHON:-tools/.venv/bin/python}"
if "$PY" -c "import manifold3d" 2>/dev/null; then
    "$(dirname "$K")/python.exe" "$(win tools/board_shapes.py)" "$(win turret-pdb.kicad_pcb)" "$(win "$OUT/shapes.json")"
    "$PY" tools/plates.py "$OUT/shapes.json" "$OUT"
else
    echo "Skipping the print plates: python3 -m venv tools/.venv && tools/.venv/bin/pip install manifold3d numpy"
fi
"$K" pcb export pdf --mode-single --layers F.Cu,Edge.Cuts,F.Fab --sketch-pads-on-fab-layers \
    -o "$(win "$OUT/front-copper.pdf")" turret-pdb.kicad_pcb >/dev/null
"$K" pcb export pdf --mode-single --mirror --layers B.Cu,Edge.Cuts \
    -o "$(win "$OUT/back-copper-mirrored.pdf")" turret-pdb.kicad_pcb >/dev/null
"$K" pcb export pdf --mode-single --layers F.SilkS,F.Fab,Edge.Cuts \
    -o "$(win "$OUT/assembly.pdf")" turret-pdb.kicad_pcb >/dev/null
"$K" pcb render --side top --width 1600 --height 1520 --background opaque \
    -o "$(win "$OUT/render-top.png")" turret-pdb.kicad_pcb >/dev/null
"$K" pcb render --side bottom --width 1600 --height 1520 --background opaque \
    -o "$(win "$OUT/render-bottom.png")" turret-pdb.kicad_pcb >/dev/null
ls -1 "$OUT"
