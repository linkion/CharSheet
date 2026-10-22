"""Spells: "Spell Descriptions" in the Spells chapter."""

import re

from common import ABILITIES, DAMAGE_TYPES, SCHOOLS, dice, make_id, summary
from markdown import plain

HEADER = re.compile(r"^(?:Level (\d) (\w+)|(\w+) Cantrip) \(([^)]*)\)$")
UNITS = {"round": "round", "minute": "minute", "hour": "hour", "day": "day"}


def _amount_unit(text):
    m = re.search(r"(\d+)\s+(round|minute|hour|day)s?", text)
    return (int(m.group(1)), UNITS[m.group(2)]) if m else (None, None)


def casting_time(text):
    ritual = bool(re.search(r"\bor Ritual\b", text))
    main = re.sub(r"\s+or Ritual\b", "", text)
    out = {"text": text}
    head, _, trigger = main.partition(", which you take ")
    if head.startswith("Bonus Action"):
        out["activation"] = "bonus_action"
    elif head.startswith("Reaction"):
        out["activation"] = "reaction"
    elif head.startswith("Action"):
        out["activation"] = "action"
    else:
        amount, unit = _amount_unit(head)
        out.update(activation="time", amount=amount, unit=unit)
    if trigger:
        out["trigger"] = "you take " + trigger
    return out, ritual


def spell_range(text):
    t = text.lower()
    if t in ("self", "touch", "sight", "unlimited", "special"):
        return {"kind": t, "text": text}
    m = re.match(r"([\d,]+) (feet|foot|miles?)", t)
    if m:
        n = int(m.group(1).replace(",", ""))
        unit = "feet" if m.group(2).startswith("f") else "miles"
        return {"kind": unit, "value": n, "text": text}
    m = re.match(r"self \((\d+)-foot (\w+)\)", t)
    if m:
        return {"kind": "self", "area": {"shape": m.group(2), "size": int(m.group(1))}, "text": text}
    return {"kind": "special", "text": text}


def components(text):
    out = {"v": False, "s": False, "m": False, "text": text}
    head, _, material = text.partition("(")
    letters = {c.strip() for c in head.split(",")}
    out["v"], out["s"], out["m"] = "V" in letters, "S" in letters, "M" in letters
    if material:
        material = material.rsplit(")", 1)[0].strip()
        out["material"] = material
        m = re.search(r"worth (?:at least )?([\d,]+)\+? GP", material)
        if m:
            out["cost_gp"] = int(m.group(1).replace(",", ""))
        out["consumed"] = "consume" in material
    return out


def duration(text):
    t = text.strip()
    conc = t.lower().startswith("concentration")
    rest = re.sub(r"^Concentration,?\s*", "", t, flags=re.I)
    out = {"concentration": conc, "text": t}
    low = rest.lower()
    if low.startswith("instantaneous"):
        out["kind"] = "instantaneous"
    elif low.startswith("until dispelled"):
        out["kind"] = "until_dispelled"
        out["or_triggered"] = "triggered" in low
    elif low.startswith("special"):
        out["kind"] = "special"
    else:
        amount, unit = _amount_unit(low)
        out.update(kind="timed", amount=amount, unit=unit, up_to=low.startswith("up to"))
    return out


def parse(section, classes_by_spell):
    """Every spell under "Spell Descriptions"."""
    out = []
    for node in section.children:
        out.append(parse_spell(node, classes_by_spell))
    return out


def parse_spell(node, classes_by_spell):
    name = node.title
    meta, body = {}, []
    header = None
    for t in node.tokens:
        text = plain(t.md) if t.kind == "para" else ""
        # "Component:" is a typo in the PDF (Barkskin).
        m = re.match(r"^(Casting Time|Range|Components?|Duration):\s*(.*)$", text)
        if header is None and t.kind == "para" and HEADER.match(text):
            header = HEADER.match(text)
        elif m and len(meta) < 4 and not body:
            meta[m.group(1).replace("Component", "Components").replace("ss", "s")] = m.group(2)
        else:
            body.append(t)
    missing = {"Casting Time", "Range", "Components", "Duration"} - set(meta)
    if header is None or missing:
        raise ValueError(f"spell {name!r}: no level/school line or missing {missing}: {[t.md for t in node.tokens[:6]]}")
    level = int(header.group(1)) if header.group(1) else 0
    school = (header.group(2) or header.group(3)).lower()
    assert school in SCHOOLS, (name, school)
    listed = [make_id("class", c.strip()) for c in header.group(4).split(",")]

    ct, ritual = casting_time(meta["Casting Time"])
    dur = duration(meta["Duration"])
    body_md = "\n\n".join(t.to_md() for t in body)
    higher, upgrade = None, None
    for t in body:
        if t.kind != "para":
            continue
        m = re.match(r"^\*\*\*(Using a Higher-Level Spell Slot|Cantrip Upgrade)\.\*\*\*\s*(.*)$", t.md, re.S)
        if m and m.group(1).startswith("Using"):
            higher = m.group(2)
        elif m:
            upgrade = m.group(2)
    text = plain(body_md)
    rec = {
        "id": make_id("spell", name),
        "name": name,
        "category": "spell",
        "page": node.page,
        "level": level,
        "school": school,
        "classes": sorted(set(listed) | set(classes_by_spell.get(make_id("spell", name), []))),
        "casting_time": ct,
        "ritual": ritual,
        "range": spell_range(meta["Range"]),
        "components": components(meta["Components"]),
        "duration": dur,
        "concentration": dur["concentration"],
    }
    if higher:
        rec["higher_levels"] = higher
    if upgrade:
        rec["cantrip_upgrade"] = upgrade
    saves = [a for a in ABILITIES if re.search(rf"\b{a} saving throw", text, re.I)]
    if saves:
        rec["saves"] = saves
    attack = re.search(r"\b(melee|ranged) spell attack", text, re.I)
    if attack:
        rec["attack"] = attack.group(1).lower()
    # The damage rolls the description names, in order ("8d6 Fire damage"), for a roller.
    damage = []
    for m in re.finditer(r"\b(\d+d\d+(?: \+ \d+)?) (\w+) damage", text):
        roll = {"dice": dice(m.group(1)), "type": m.group(2).lower()}
        if roll["type"] in DAMAGE_TYPES and roll not in damage:
            damage.append(roll)
    if damage:
        rec["damage"] = damage
        rec["damage_types"] = sorted({d["type"] for d in damage})
    rec["summary"] = summary(body_md)
    rec["markdown"] = node.md()
    return rec
