"""Monster and animal stat blocks."""

import re
from fractions import Fraction

from common import (
    ABILITY_ABBR, ability_tables, CONDITIONS, CREATURE_TYPES, DAMAGE_TYPES, SIZES, dice, key, make_id, signed, split_list,
)
from markdown import plain

SECTIONS = {
    "Traits": "traits",
    "Actions": "actions",
    "Bonus Actions": "bonus_actions",
    "Reactions": "reactions",
    "Legendary Actions": "legendary_actions",
}
NUM = r"[+−-]?\d+"


def parse(chapter, group):
    out = []
    for node in chapter.walk():
        if node.role != "statblock":
            continue
        family = node.parent.title if node.parent.level == 2 and node.parent.title != node.title else None
        out.append(parse_statblock(node, group, family))
    return out


def type_line(text):
    """"Medium or Small Humanoid (Wizard), Neutral" and "Medium Swarm of Tiny Beasts, Unaligned"."""
    head, _, alignment = text.rpartition(", ")
    m = re.match(r"^((?:\w+)(?: or \w+)*) (Swarm of \w+ )?(\w+)(?: \(([^)]*)\))?$", head)
    if not m:
        raise ValueError(f"type line {text!r}")
    sizes = [s.lower() for s in m.group(1).split(" or ")]
    ctype = m.group(3).lower().rstrip("s") if m.group(2) else m.group(3).lower()
    assert all(s in SIZES for s in sizes) and ctype in CREATURE_TYPES, text
    out = {"size": sizes, "creature_type": ctype, "tags": [t.strip() for t in (m.group(4) or "").split(",") if t.strip()]}
    if m.group(2):
        out["swarm"] = {"size": m.group(2).split()[2].lower()}
    out["alignment"] = alignment
    return out


def speed(text):
    out = {}
    for part in text.split(","):
        part = part.strip()
        m = re.match(r"^(?:(\w+(?: or \w+)?) )?(\d+) ft\.(.*)$", part)
        if not m:
            out.setdefault("notes", []).append(part)
            continue
        modes = (m.group(1) or "walk").lower().split(" or ")
        rest = m.group(3).strip()
        if rest and "hover" not in rest:
            # "40 ft. (bear form only)": a conditional speed, kept as text.
            out.setdefault("notes", []).append(part)
            continue
        for mode in modes:
            out[mode] = int(m.group(2))
        if "hover" in rest:
            out["hover"] = True
    return out


def senses(text):
    out = {}
    for part in re.split(r"[;,]\s*", text):
        m = re.match(r"^(\w+) (\d+) ft\.(?: \(([^)]*)\))?$", part.strip())
        p = re.match(r"^Passive Perception (\d+)$", part.strip())
        if p:
            out["passive_perception"] = int(p.group(1))
        elif m:
            out[m.group(1).lower()] = int(m.group(2))
            if m.group(3):
                out[f"{m.group(1).lower()}_note"] = m.group(3)
        elif part.strip():
            out.setdefault("notes", []).append(part.strip())
    return out


def challenge(text):
    """"10 (XP 5,900, or 7,200 in lair; PB +4)"."""
    # A few stat blocks print "700 XP" rather than "XP 700".
    text = re.sub(r"\(([\d,]+) XP", r"(XP \1", text)
    m = re.match(r"^([\d/]+) \(XP ([\d,]+)(?:,? or ([\d,]+)(?: in lair)?)?; PB ([+−-]\d+)\)$", text)
    if not m:
        raise ValueError(f"CR {text!r}")
    out = {
        "value": float(Fraction(m.group(1))),
        "text": m.group(1),
        "xp": int(m.group(2).replace(",", "")),
        "pb": signed(m.group(4)),
    }
    if m.group(3):
        out["xp_lair" if "lair" in text else "xp_alt"] = int(m.group(3).replace(",", ""))
    return out


def defences(text):
    """"Poison, Thunder; Exhaustion, Grappled" -> damage types and conditions."""
    dmg, cond, other = [], [], []
    for item in re.split(r"[;,]\s*", text):
        k = key(item)
        if k in DAMAGE_TYPES:
            dmg.append(k)
        elif k in CONDITIONS:
            cond.append(k)
        elif item.strip():
            other.append(item.strip())
    return dmg, cond, other


def action(md):
    """One trait or action: its name, usage limit and the numbers in it."""
    m = re.match(r"^\*\*\*(.+?)\.\*\*\*\s*(.*)$", md, re.S)
    if not m:
        return None
    full, body = m.group(1), m.group(2)
    name, usage = full, None
    u = re.match(r"^(.*?) \(([^()]*(?:Day|Recharge|Rest|Costs|Turn|Lair)[^()]*)\)$", full)
    if u:
        name, usage = u.group(1), u.group(2)
    out = {"name": name}
    if usage:
        out["usage"] = usage
    text = plain(body)
    atk = re.match(r"^(Melee|Ranged|Melee or Ranged) Attack Roll: ([+−-]\d+)(?:, reach (\d+) f(?:ee)?t\.?)?(?:,? (?:or )?range (\d+)(?:/(\d+))? f(?:ee)?t\.?)?", text)
    if atk:
        a = {"kind": key(atk.group(1)), "bonus": signed(atk.group(2))}
        if atk.group(3):
            a["reach"] = int(atk.group(3))
        if atk.group(4):
            a["range"] = int(atk.group(4))
            if atk.group(5):
                a["long_range"] = int(atk.group(5))
        out["attack"] = a
    sv = re.search(r"\b(Strength|Dexterity|Constitution|Intelligence|Wisdom|Charisma) Saving Throw: DC (\d+)", text)
    if sv:
        out["save"] = {"ability": sv.group(1).lower(), "dc": int(sv.group(2))}
    dmg = []
    for d in re.finditer(r"\b(\d+) \((\d+d\d+(?: [+−-] \d+)?)\) (\w+) damage", text):
        if d.group(3).lower() in DAMAGE_TYPES:
            dmg.append({"average": int(d.group(1)), "dice": dice(d.group(2)), "type": d.group(3).lower()})
    for d in re.finditer(r"Hit: (\d+) (\w+) damage", text):
        if d.group(2).lower() in DAMAGE_TYPES:
            dmg.insert(0, {"average": int(d.group(1)), "dice": None, "type": d.group(2).lower()})
    if dmg:
        out["damage"] = dmg
    out["markdown"] = body.strip()
    return out


def entries(tokens):
    """Groups a stat block section's paragraphs into named entries."""
    out, intro = [], []
    for t in tokens:
        md = t.to_md()
        entry = action(md) if t.kind == "para" else None
        if entry:
            out.append(entry)
        elif out:
            out[-1]["markdown"] += "\n\n" + md
        else:
            intro.append(md)
    return out, intro


def parse_statblock(node, group, family):
    rec = {
        "id": make_id("monster", node.title),
        "name": node.title,
        "category": "monster",
        "group": group,
        "page": node.page,
    }
    if family:
        rec["family"] = family
    abilities = {}
    labels = {}
    first = True
    for t in node.tokens:
        text = plain(t.md)
        if first:
            rec.update(type_line(text))
            first = False
            continue
        for m in re.finditer(rf"\b(Str|Dex|Con|Int|Wis|Cha) (\d+) ({NUM}) ({NUM})", text.replace("−", "-")):
            abilities[ABILITY_ABBR[m.group(1).lower()][:3]] = {
                "score": int(m.group(2)), "mod": int(m.group(3)), "save": int(m.group(4)),
            }
        parts = re.split(r"\*\*([A-Z]\w*)\*\*", t.md)
        for label, value in zip(parts[1::2], parts[2::2]):
            labels[label] = plain(value)
    rec["ac"] = int(labels["AC"].split()[0])
    ini = re.match(r"([+−-]\d+) \((\d+)\)", labels["Initiative"])
    rec["initiative"] = {"mod": signed(ini.group(1)), "score": int(ini.group(2))}
    hp = re.match(r"(\d+)(?: \(([^)]*)\))?", labels["HP"])
    rec["hp"] = {"average": int(hp.group(1)), "formula": dice(hp.group(2)) if hp.group(2) else None}
    rec["speed"] = speed(labels["Speed"])
    rec["abilities"] = abilities
    if "Skills" in labels:
        rec["skills"] = {
            key(m.group(1)): signed(m.group(2))
            for m in re.finditer(r"([A-Z][\w ]+?) ([+−-]\d+)", labels["Skills"])
        }
    for label in ("Resistances", "Vulnerabilities"):
        if label in labels:
            dmg, _, other = defences(labels[label])
            rec[label.lower()] = {"damage": dmg, "notes": other} if other else {"damage": dmg}
    if "Immunities" in labels:
        dmg, cond, other = defences(labels["Immunities"])
        rec["immunities"] = {"damage": dmg, "conditions": cond}
        if other:
            rec["immunities"]["notes"] = other
    if "Gear" in labels:
        rec["gear"] = split_list(labels["Gear"])
    rec["senses"] = senses(labels["Senses"])
    rec["languages"] = labels["Languages"]
    rec["cr"] = challenge(labels["CR"])
    for child in node.children:
        field = SECTIONS[child.title]
        items, intro = entries(child.tokens)
        if field == "legendary_actions":
            uses = re.search(r"Legendary Action Uses: (\d+)(?: \((\d+) in Lair\))?", plain("\n".join(intro)))
            rec[field] = {
                "uses": int(uses.group(1)) if uses else None,
                "uses_lair": int(uses.group(2)) if uses and uses.group(2) else None,
                "intro": "\n\n".join(intro),
                "actions": items,
            }
        else:
            rec[field] = items
    rec["summary"] = f"{plain(node.tokens[0].md)}; CR {rec['cr']['text']}"
    rec["markdown"] = ability_tables(node.md())
    return rec
