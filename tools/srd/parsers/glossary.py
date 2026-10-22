"""The Rules Glossary, and the conditions drawn from it."""

import re

from common import make_id, summary

TAGGED = re.compile(r"^(.*?) \[(.+)\]$")


def parse(chapter):
    out = []
    nodes = chapter.child("Rules Definitions").children
    names = {}
    for node in nodes:
        m = TAGGED.match(node.title)
        name, tag = (m.group(1), m.group(2)) if m else (node.title, None)
        names[name.lower()] = make_id("glossary", name)
        md = node.md()
        rec = {
            "id": make_id("glossary", name),
            "name": name,
            "category": "glossary",
            "page": node.page,
            "tag": tag.lower().replace(" ", "_") if tag else None,
        }
        # Every quoted name after "See also", entries and chapters alike.
        rec["see_also_text"] = re.findall(r"“([^”]+?)[,.]?”", "".join(re.findall(r"\*See also\*[^\n]*", md)))
        rec["summary"] = summary(md)
        rec["markdown"] = md
        out.append(rec)
    # Resolve "See also" names that are glossary entries; others name chapters and stay text.
    for rec in out:
        refs = rec["see_also_text"]
        rec["see_also"] = sorted({names[r.lower()] for r in refs if r.lower() in names} - {rec["id"]})
    return out


def conditions(glossary):
    """Glossary entries tagged [Condition], with their effects split out."""
    out = []
    for g in glossary:
        if g["tag"] != "condition":
            continue
        effects = []
        levels = []
        for para in g["markdown"].split("\n\n"):
            m = re.match(r"^\*\*\*(.+?)\.\*\*\*\s*(.*)$", para, re.S)
            if m:
                effects.append({"name": m.group(1), "markdown": m.group(2)})
            elif effects:
                effects[-1]["markdown"] += "\n\n" + para
        rec = {
            "id": make_id("condition", g["name"]),
            "name": g["name"],
            "category": "condition",
            "page": g["page"],
            "glossary": g["id"],
            "effects": effects,
        }
        if g["name"] == "Exhaustion":
            # "Exhaustion Levels. ... cumulative ... levels" and "D20 Tests Affected. ... 2 times your Exhaustion level"
            rec["max_level"] = 6
            rec["per_level"] = {"d20_tests": -2, "speed_ft": -5}
        rec["summary"] = g["summary"]
        rec["markdown"] = g["markdown"]
        out.append(rec)
    return out
