#!/usr/bin/env python3
"""Turns the SRD 5.2.1 PDF into structured JSON.

Usage: python3 -I tools/srd/build.py SRD_CC_v5.2.1.pdf data/srd
Needs mutool (MuPDF) on the PATH; everything else is the standard library. The output is
deterministic apart from manifest.json's generated_at, so rerunning it changes nothing.
"""

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import layout  # noqa: E402
from common import ability_tables  # noqa: E402
import markdown  # noqa: E402
import schema  # noqa: E402
import sections  # noqa: E402
from parsers import classes, equipment, glossary, magic_items, monsters, origins, rules, spells  # noqa: E402
from parsers import feats as feats_parser  # noqa: E402

ATTRIBUTION = (
    "This work includes material from the System Reference Document 5.2.1 (“SRD 5.2.1”) "
    "by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is "
    "licensed under the Creative Commons Attribution 4.0 International License, available at "
    "https://creativecommons.org/licenses/by/4.0/legalcode."
)

FIRST_PAGE = 5  # pages 1-4 are the legal notice and contents


def load_tree(pdf):
    lines = layout.document(pdf, FIRST_PAGE)
    tokens = markdown.Tokenizer(lines).tokens()
    return sections.build(tokens)


def overrides():
    path = Path(__file__).parent / "overrides.json"
    return json.loads(path.read_text()) if path.exists() else {}


def apply_overrides(files, patches):
    """Merges hand-written fixes into records, keyed by record id."""
    by_id = {r["id"]: r for recs in files.values() for r in recs}
    for rid, patch in patches.items():
        if rid.startswith("_"):
            continue
        if rid not in by_id:
            raise KeyError(f"override for unknown record {rid}")
        merge(by_id[rid], {k: v for k, v in patch.items() if not k.startswith("_")})


def merge(target, patch):
    """Deep-merges patch into target: nested objects merge, anything else replaces."""
    for k, v in patch.items():
        if isinstance(v, dict) and isinstance(target.get(k), dict):
            merge(target[k], v)
        else:
            target[k] = v


def index_rows(files):
    rows = []
    for name, recs in files.items():
        for r in recs:
            rows.append({
                "id": r["id"],
                "name": r["name"],
                "category": r["category"],
                "aliases": r.get("aliases", []),
                "keywords": keywords(r),
                "page": r["page"],
                "file": f"{name}.json",
                "summary": r.get("summary", ""),
            })
    rows.sort(key=lambda r: (r["category"], r["name"].lower(), r["id"]))
    return rows


def keywords(r):
    c = r["category"]
    out = []
    if c == "spell":
        out = ["cantrip" if r["level"] == 0 else f"level {r['level']}", r["school"]]
        out += [x.split(":")[1] for x in r["classes"]]
        out += ["ritual"] * r["ritual"] + ["concentration"] * r["concentration"]
    elif c == "monster":
        out = [r["creature_type"], f"cr {r['cr']['value']}", *r["size"], r["group"]]
    elif c == "magic_item":
        out = [r["item_type"], *(r["rarity"] if isinstance(r["rarity"], list) else [r["rarity"]])]
        if r["attunement"]["required"]:
            out.append("attunement")
    elif c in ("weapon", "armor"):
        out = [r.get("weapon_category") or r.get("armor_category")]
    elif c == "feat":
        out = [r["feat_category"]]
    elif c == "glossary":
        out = [r["tag"]] if r.get("tag") else []
    elif c == "rule":
        out = r["path"][:-1]
    return [k for k in out if k]


def main(pdf, out_dir):
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    root = load_tree(pdf)

    class_recs, subclass_recs, class_spells = classes.parse(root.child("Classes"))
    files = {
        "rules": rules.parse(root),
        "glossary": glossary.parse(root.child("Rules Glossary")),
        "classes": class_recs,
        "subclasses": subclass_recs,
        "backgrounds": origins.backgrounds(root.child("Character Origins")),
        "species": origins.species(root.child("Character Origins")),
        "feats": feats_parser.parse(root.child("Feats")),
        "spells": spells.parse(root.child("Spells").child("Spell Descriptions"), class_spells),
        "magic_items": magic_items.parse(root.child("Magic Items").child("Magic Items A–Z")),
        "monsters": monsters.parse(root.child("Monsters A–Z"), "monster")
        + monsters.parse(root.child("Animals"), "animal"),
    }
    files["conditions"] = glossary.conditions(files["glossary"])
    files.update(equipment.parse(root.child("Equipment")))
    for recs in files.values():
        for r in recs:
            r["markdown"] = ability_tables(r["markdown"])  # stat blocks inside spells and items
    apply_overrides(files, overrides())

    counts = {}
    for name, recs in sorted(files.items()):
        if name != "rules":  # rules stay in book order
            recs.sort(key=lambda r: (r["name"].lower(), r["id"]))
        write(out / f"{name}.json", recs)
        counts[f"{name}.json"] = len(recs)
    (out / "schema").mkdir(exist_ok=True)
    for name, doc in schema.documents().items():
        write(out / "schema" / name, doc)
    index = index_rows(files)
    write(out / "index.json", index)
    counts["index.json"] = len(index)
    write(out / "manifest.json", {
        "source": "System Reference Document 5.2.1",
        "version": "5.2.1",
        "license": "CC-BY-4.0",
        "attribution": ATTRIBUTION,
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "files": counts,
    })
    print(json.dumps(counts, indent=2))


def write(path, data):
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
