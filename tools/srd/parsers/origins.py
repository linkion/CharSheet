"""Character Origins: backgrounds and species."""

import re

from common import ABILITIES, CREATURE_TYPES, SIZES, fields, key, make_id, split_list
from markdown import plain


def backgrounds(chapter):
    out = []
    for node in chapter.child("Character Backgrounds").child("Background Descriptions").children:
        f = fields("\n\n".join(t.md for t in node.tokens))
        feat = re.sub(r"\s*\(see [^)]*\)", "", f["Feat"]).strip()
        tool = plain(f["Tool Proficiency"])
        rec = {
            "id": make_id("background", node.title),
            "name": node.title,
            "category": "background",
            "page": node.page,
            "ability_scores": [a.lower() for a in split_list(f["Ability Scores"])],
            "feat": {"name": feat, "id": make_id("feat", re.sub(r"\s*\(.*\)$", "", feat))},
            "skill_proficiencies": [key(s) for s in split_list(f["Skill Proficiencies"])],
            "tool_proficiency": re.sub(r"\s*\(see [^)]*\)", "", tool),
            "equipment": f["Equipment"],
        }
        assert all(a in ABILITIES for a in rec["ability_scores"]), rec["ability_scores"]
        rec["summary"] = (
            f"Abilities: {', '.join(a.title() for a in rec['ability_scores'])}; feat: {feat}; "
            f"skills: {plain(f['Skill Proficiencies'])}"
        )
        rec["markdown"] = node.md()
        out.append(rec)
    return out


def species(chapter):
    out = []
    for node in chapter.child("Character Species").child("Species Descriptions").children:
        head = "\n\n".join(t.md for t in node.tokens[:2])
        f = fields(head)
        size_text = f["Size"]
        sizes = [s for s in re.findall(r"\w+", re.sub(r"\(.*?\)", "", size_text).lower()) if s in SIZES]
        speed = re.match(r"(\d+) feet", f["Speed"])
        traits = []
        intro = []
        for t in node.tokens[2:]:
            m = re.match(r"^\*\*\*(.+?)\.\*\*\*\s*(.*)$", t.md, re.S) if t.kind == "para" else None
            if m:
                traits.append({"name": m.group(1), "markdown": m.group(2)})
            elif traits:
                traits[-1]["markdown"] += "\n\n" + t.to_md()
            else:
                intro.append(t.to_md())
        rec = {
            "id": make_id("species", node.title),
            "name": node.title,
            "category": "species",
            "page": node.page,
            "creature_type": f["Creature Type"].lower(),
            "size": sizes,
            "size_text": size_text,
            "speed": int(speed.group(1)),
            "traits": traits,
        }
        assert rec["creature_type"] in CREATURE_TYPES
        rec["summary"] = f"{f['Creature Type']}, {size_text}; Speed {rec['speed']} feet; traits: " + ", ".join(
            t["name"] for t in traits
        )
        rec["markdown"] = node.md()
        out.append(rec)
    return out
