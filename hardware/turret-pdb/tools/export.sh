#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

K="${KICAD_CLI:-/mnt/c/Users/$(cmd.exe /c 'echo %USERNAME%' 2>/dev/null | tr -d '\r')/AppData/Local/Programs/KiCad/10.0/bin/kicad-cli.exe}"
PY="${PLATES_PYTHON:-tools/.venv/bin/python}"
win() { if command -v wslpath >/dev/null; then wslpath -w "$1"; else echo "$1"; fi; }

export_board() {
    local b=$1 OUT=out/$1 P=$1/turret-$1
    mkdir -p "$OUT"
    echo "== $b"
    "$K" sch erc --format json --exit-code-violations -o "$(win "$OUT/erc.json")" "$P.kicad_sch" >/dev/null || true
    "$K" pcb drc --format json --schematic-parity --refill-zones --save-board \
        -o "$(win "$OUT/drc.json")" "$P.kicad_pcb" >/dev/null || true

    local ok=0
    python3 - "$OUT" <<'PY' || ok=1
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

    if "$PY" -c "import manifold3d" 2>/dev/null; then
        "$(dirname "$K")/python.exe" "$(win tools/board_shapes.py)" "$(win "$P.kicad_pcb")" "$(win "$OUT/shapes.json")" || ok=1
        "$PY" tools/plates.py "$OUT/shapes.json" "$OUT" || ok=1
    else
        echo "Skipping the print plates: python3 -m venv tools/.venv && tools/.venv/bin/pip install manifold3d numpy"
    fi
    "$K" pcb export pdf --mode-single --layers F.Cu,Edge.Cuts,F.Fab --sketch-pads-on-fab-layers \
        -o "$(win "$OUT/front-copper.pdf")" "$P.kicad_pcb" >/dev/null || ok=1
    "$K" pcb export pdf --mode-single --mirror --layers B.Cu,Edge.Cuts \
        -o "$(win "$OUT/back-copper-mirrored.pdf")" "$P.kicad_pcb" >/dev/null || ok=1
    "$K" pcb export pdf --mode-single --layers F.SilkS,F.Fab,Edge.Cuts \
        -o "$(win "$OUT/assembly.pdf")" "$P.kicad_pcb" >/dev/null || ok=1
    rm -rf "$OUT/gerbers"
    mkdir -p "$OUT/gerbers"
    "$K" pcb export gerbers --no-x2 --subtract-soldermask \
        --layers F.Cu,B.Cu,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts \
        -o "$(win "$OUT/gerbers")" "$P.kicad_pcb" >/dev/null || ok=1
    "$K" pcb export drill --excellon-separate-th --excellon-units mm --excellon-oval-format alternate \
        -o "$(win "$OUT/gerbers")/" "$P.kicad_pcb" >/dev/null || ok=1
    python3 -c 'import pathlib, sys, zipfile
src = pathlib.Path(sys.argv[1])
with zipfile.ZipFile(sys.argv[2], "w", zipfile.ZIP_DEFLATED) as z:
    for f in sorted(src.iterdir()):
        z.write(f, f.name)' "$OUT/gerbers" "$OUT/turret-$b-gerbers.zip" || ok=1
    "$K" pcb render --side top --width 1600 --height 1520 --background opaque \
        -o "$(win "$OUT/render-top.png")" "$P.kicad_pcb" >/dev/null || ok=1
    "$K" pcb render --side bottom --width 1600 --height 1520 --background opaque \
        -o "$(win "$OUT/render-bottom.png")" "$P.kicad_pcb" >/dev/null || ok=1
    return $ok
}

boards=("$@")
if [ ${#boards[@]} -eq 0 ]; then
    for b in power logic; do [ -f "$b/turret-$b.kicad_pcb" ] && boards+=("$b"); done
fi
failed=()
for b in "${boards[@]}"; do export_board "$b" || failed+=("$b"); done
if [ ${#failed[@]} -gt 0 ]; then
    echo "Failed: ${failed[*]}"
    exit 1
fi
