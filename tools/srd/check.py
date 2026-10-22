#!/usr/bin/env python3
"""Checks data/srd: schemas, ids and references, and completeness against the PDF.

Usage: python3 -I tools/srd/check.py data/srd [SRD_CC_v5.2.1.pdf]
With the PDF it also checks every monster in the book's Index of Stat Blocks was found.
Exits non-zero and lists the problems if any check fails.
"""

import json
import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import schema  # noqa: E402

EXPECTED = {"classes": 12, "subclasses": 12, "species": 9, "backgrounds": 4, "conditions": 15}

problems = []


def problem(msg):
    problems.append(msg)


def load(data):
    files = {}
    for name in schema.SCHEMAS:
        files[name] = json.loads((data / f"{name}.json").read_text())
    return files


def check_schemas(files, index):
    for name, recs in files.items():
        item = schema.SCHEMAS[name]
        for r in recs:
            for err in schema.validate(r, item, f"{name}:{r.get('id')}"):
                problem(err)
    for r in index:
        for err in schema.validate(r, schema.INDEX, f"index:{r.get('id')}"):
            problem(err)


def check_ids(files, index):
    ids = Counter(r["id"] for recs in files.values() for r in recs)
    for i, n in ids.items():
        if n > 1:
            problem(f"duplicate id {i}")
    known = set(ids)
    # Every string that looks like an id must name a record.
    pattern = re.compile(r"^(spell|class|subclass|feat|glossary|weapon_property|mastery_property|rule|condition):[a-z0-9/-]+$")

    def refs(value, where):
        if isinstance(value, dict):
            for k, v in value.items():
                if k != "id":
                    refs(v, where)
        elif isinstance(value, list):
            for v in value:
                refs(v, where)
        elif isinstance(value, str) and pattern.match(value) and value not in known:
            problem(f"{where}: unknown reference {value}")

    for recs in files.values():
        for r in recs:
            refs(r, r["id"])
    if {r["id"] for r in index} != known:
        problem("index.json doesn't list exactly the records in the data files")


def check_counts(files):
    for name, n in EXPECTED.items():
        if len(files[name]) != n:
            problem(f"{name}: expected {n} records, found {len(files[name])}")


def check_consistency(files):
    spells = {s["id"]: s for s in files["spells"]}
    for c in files["classes"]:
        for sid in c["spell_list"]:
            if c["id"] not in spells[sid]["classes"]:
                problem(f"{sid} is on the {c['name']} list but doesn't name the class")
    on_lists = {sid for c in files["classes"] for sid in c["spell_list"]}
    for s in files["spells"]:
        # The SRD's class spell lists leave out Phantasmal Force, though the spell names
        # Bard, Sorcerer and Wizard.
        if s["id"] not in on_lists and s["id"] != "spell:phantasmal-force":
            problem(f"{s['id']} is on no class spell list")
        if s["level"] == 0 and "higher_levels" in s:
            problem(f"{s['id']}: a cantrip with a higher-level slot rule")
    for c in files["classes"]:
        levels = [r["level"] for r in c["features_table"]["rows"]]
        if levels != list(range(1, 21)):
            problem(f"{c['id']}: features table levels {levels}")
        named = {f["name"] for f in c["features"]}
        for row in c["features_table"]["rows"]:
            for f in row["features"]:
                f = re.sub(r" \(.*\)$", "", f)  # "Action Surge (two uses)"
            if f not in named and f != "Subclass feature" and not f.endswith("Subclass"):
                    problem(f"{c['id']}: level {row['level']} feature {f!r} has no description")
    for m in files["monsters"]:
        for a in m["abilities"].values():
            if a["mod"] != (a["score"] - 10) // 2:
                problem(f"{m['id']}: modifier {a['mod']} doesn't match score {a['score']}")
            if a["save"] < a["mod"] or (a["save"] - a["mod"]) not in (0, m["cr"]["pb"]):
                problem(f"{m['id']}: save {a['save']} is neither the modifier {a['mod']} nor proficient")
        for field in ("traits", "actions", "bonus_actions", "reactions"):
            for act in m.get(field, []):
                for d in act.get("damage", []):
                    if d["dice"] and not re.fullmatch(r"\d+d\d+([+-]\d+)?", d["dice"]):
                        problem(f"{m['id']} {act['name']}: dice {d['dice']}")
    for i in files["magic_items"]:
        if not 209 <= i["page"] <= 253:
            problem(f"{i['id']}: page {i['page']} outside Magic Items A–Z")
    for r in (r for recs in files.values() for r in recs):
        if not r["markdown"].strip() and not r.get("children") and r["category"] not in ("mount", "vehicle", "service"):
            problem(f"{r['id']}: no text")
        # A word split at a line break and not rejoined: "dam- age".
        for m in re.finditer(r"\b[a-z]+- (?!and\b|or\b|to\b)[a-z]{2,}\b", r["markdown"]):
            problem(f"{r['id']}: possible broken hyphenation {m.group(0)!r}")


def stat_block_index(pdf):
    """(name, page) for each entry of the PDF's Index of Stat Blocks."""
    import layout

    out = []
    for _, blocks in layout.read_pages(pdf, 2, 4):
        for line in layout.order(blocks):
            # The index has three columns, so one line can hold several entries.
            for part in line.text.split("\t"):
                m = re.match(r"^(.+?)\s*\.{2,}\s*(\d+)$", part.strip())
                # Contents entries share these pages; stat blocks start on page 258.
                if m and int(m.group(2)) >= 258 and m.group(1).strip() not in ("Monsters A–Z", "Animals"):
                    out.append((m.group(1).strip(), int(m.group(2))))
    return out


def check_index(files, pdf):
    monsters = {m["name"]: m for m in files["monsters"]}
    entries = stat_block_index(pdf)
    if len(entries) < 300:
        problem(f"read only {len(entries)} entries from the Index of Stat Blocks")
    for name, page in entries:
        m = monsters.get(name)
        if m is None:
            problem(f"Index of Stat Blocks lists {name!r} (p{page}), not in monsters.json")
        elif m["page"] != page:
            problem(f"{m['id']}: page {m['page']}, index says {page}")
    listed = {n for n, _ in entries}
    for name in monsters:
        if name not in listed:
            problem(f"monsters.json has {name!r}, not in the Index of Stat Blocks")


def main(data, pdf=None):
    data = Path(data)
    files = load(data)
    index = json.loads((data / "index.json").read_text())
    check_schemas(files, index)
    check_ids(files, index)
    check_counts(files)
    check_consistency(files)
    if pdf:
        check_index(files, pdf)
    for p in problems:
        print(p)
    total = sum(len(r) for r in files.values())
    print(f"{len(problems)} problems in {total} records")
    return 1 if problems else 0


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    sys.exit(main(*sys.argv[1:]))
