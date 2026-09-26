#!/usr/bin/env python3
"""Anonymise des fichiers Winamax (mains + summary) de façon déterministe.

Usage : python tools/anonymize.py <hero_pseudo> <fichier_ou_dossier>... --out fixtures/winamax/<sous-dossier>
- Le Hero devient "Hero" ; les adversaires deviennent P0001, P0002... (dans l'ordre d'apparition).
- La table de correspondance est commune à tous les fichiers traités en une fois.
- Rien d'autre n'est modifié (montants, cartes, IDs, horodatages, octets de fin de fichier).
Implémentation de référence : Claude Code pourra la porter en Rust (M0-4).
"""
import re, sys, pathlib, argparse

SEAT = re.compile(r"^Seat \d+: (.+?) \(\d+(?:, [\d.]+€ bounty)?\)$", re.M)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("hero"); ap.add_argument("paths", nargs="+"); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    files = []
    for p in map(pathlib.Path, a.paths):
        files += sorted(p.rglob("*.txt")) if p.is_dir() else [p]
    mapping = {a.hero: "Hero"}
    texts = {f: f.read_text(encoding="utf-8") for f in files}
    for t in texts.values():
        for name in SEAT.findall(t):
            if name not in mapping:
                mapping[name] = f"P{len(mapping):04d}"
    names = sorted(mapping, key=len, reverse=True)
    rx = re.compile("|".join(re.escape(n) for n in names))
    out = pathlib.Path(a.out); out.mkdir(parents=True, exist_ok=True)
    for f, t in texts.items():
        # remplacement uniquement en position de pseudo : début de ligne, après "Seat N: ", "Dealt to ", "Player : "
        def sub_line(line):
            for prefix in (r"^(Seat \d+: )", r"^(Dealt to )", r"^(Player : )", r"^()"):
                m = re.match(prefix + "(" + rx.pattern + r")(?=[ \n]|$)", line)
                if m:
                    return m.group(1) + mapping[m.group(2)] + line[m.end():]
            return line
        (out / f.name).write_text("\n".join(sub_line(l) for l in t.split("\n")), encoding="utf-8", newline="\n")
    print(f"{len(files)} fichier(s), {len(mapping)} pseudos anonymisés -> {out}")

if __name__ == "__main__":
    main()
