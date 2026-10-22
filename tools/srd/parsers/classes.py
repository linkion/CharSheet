"""Classes and subclasses."""

import re

from common import ABILITIES, SKILLS, key, make_id, split_list, summary, table_rows
from markdown import plain

FEATURE = re.compile(r"^Level (\d+): (.+)$")
# The Warlock's Pact Magic columns, renamed so they don't clash with spell_slots.
PACT_COLUMNS = {"Spell Slots": "Pact Slots", "Slot Level": "Pact Slot Level"}


def tables(section):
    for s in section.walk():
        for t in s.tokens:
            if t.kind == "table":
                yield t


def spell_names(cell):
    """"*Aid*, *Bless*, *Cure Wound*s" -> ["Aid", "Bless", "Cure Wounds"]."""
    return [n.strip() for n in re.split(r",\s*", plain(cell)) if n.strip() and n.strip() != "—"]


def features(section):
    out = []
    for child in section.children:
        m = FEATURE.match(child.title)
        if not m:
            continue
        out.append({"level": int(m.group(1)), "name": m.group(2), "markdown": child.md()})
    return out


def core_traits(table):
    rows = {plain(r[0]): r[1] for r in table.rows}
    text = {k: plain(v) for k, v in rows.items()}
    out = {
        "primary_ability": [a.lower() for a in split_list(text["Primary Ability"]) if a.lower() in ABILITIES],
        "primary_ability_text": text["Primary Ability"],
        "hit_die": int(re.search(r"D(\d+)", text["Hit Point Die"]).group(1)),
        "saving_throws": [a.lower() for a in split_list(text["Saving Throw Proficiencies"])],
    }
    skills = text["Skill Proficiencies"]
    m = re.match(r"Choose (?:any )?(\d+)(?: skills)?:?\s*(.*)$", skills)
    if m:
        options = m.group(2)
        out["skill_choices"] = {
            "count": int(m.group(1)),
            "from": SKILLS if "any" in skills.lower() else [key(s) for s in split_list(options)],
        }
    out["weapon_proficiencies"] = text.get("Weapon Proficiencies", "")
    if "Tool Proficiencies" in text:
        out["tool_proficiencies"] = text["Tool Proficiencies"]
    out["armor_training"] = text.get("Armor Training", "")
    out["starting_equipment"] = rows["Starting Equipment"]
    return out


def features_table(table):
    """The class table: one dict per level, extra columns keyed by snake_case header.

    Spell slot columns (headed 1-9) become spell_slots: [n, …] with 0 for "—".
    """
    header = [plain(h) for h in table.header]
    rows = []
    for r in table.rows:
        cells = dict(zip(header, (plain(c) for c in r)))
        row = {
            "level": int(cells.pop("Level")),
            "proficiency_bonus": int(cells.pop("Proficiency Bonus")),
            "features": [f.strip() for f in cells.pop("Class Features").split(",") if f.strip() not in ("", "—")],
        }
        slots = [cells.pop(str(i)) for i in range(1, 10) if str(i) in cells]
        if slots:
            row["spell_slots"] = [0 if s == "—" else int(s) for s in slots]
        for k, v in cells.items():
            v = v.strip()
            k = PACT_COLUMNS.get(k, k)
            row[key(k)] = None if v == "—" else (int(v) if re.fullmatch(r"[+]?\d+", v) else v)
        rows.append(row)
    return {"columns": header, "rows": rows}


def spell_list(section, resolve):
    """Spell ids from a "<Class> Spell List" section, cantrips first."""
    out = []
    for t in section.tokens:
        if t.kind != "table":
            continue
        for row in table_rows(t):
            out.append(resolve(plain(row["Spell"])))
    return out


def option_group(section):
    """Metamagic Options, Eldritch Invocation Options: named entries with prerequisites."""
    options = []
    for child in section.children:
        opt = {"name": child.title}
        first = child.tokens[0] if child.tokens else None
        if first is not None and plain(first.md).startswith("Prerequisite:"):
            opt["prerequisite"] = plain(first.md).removeprefix("Prerequisite:").strip()
        opt["repeatable"] = "***Repeatable.***" in child.md()
        opt["markdown"] = child.md()
        options.append(opt)
    return {"name": section.title, "intro": section.own_md(), "options": options}


def subclass_spells(section, resolve):
    """Always-prepared spell tables such as "Life Domain Spells"."""
    found = []
    for t in tables(section):
        header = [plain(h) for h in t.header]
        if len(header) == 2 and header[0].endswith("Level") and "Spells" in header[1]:
            found.append(t)
    out = []
    for t in found:
        for level, cell in t.rows:
            row = {"level": int(plain(level)), "spells": [resolve(n) for n in spell_names(cell)]}
            if len(found) > 1:
                row["group"] = t.caption  # Circle of the Land: one table per land type
            out.append(row)
    return out


def parse(chapter, resolve=None):
    """Returns (classes, subclasses, {spell id: [class ids]})."""
    class_spells = {}
    resolve = resolve or (lambda name: make_id("spell", re.sub(r"\s+", " ", name.strip())))
    classes, subclasses = [], []
    for node in chapter.children:
        name = node.title
        cid = make_id("class", name)
        traits = next(t for t in node.tokens if t.kind == "table")
        becoming = node.child(f"Becoming a {name} …")
        rec = {
            "id": cid,
            "name": name,
            "category": "class",
            "page": node.page,
        }
        rec.update(core_traits(traits))
        rec["level_1"] = becoming.child("As a Level 1 Character").md()
        rec["multiclass"] = becoming.child("As a Multiclass Character").md()
        feat_section = node.child(f"{name} Class Features")
        table = next(t for t in tables(feat_section) if t.caption == f"{name} Features")
        rec["features_table"] = features_table(table)
        rec["features"] = features(feat_section)
        rec["option_groups"] = [option_group(c) for c in node.children if c.title.endswith("Options")]
        spells_section = next((c for c in node.children if c.title == f"{name} Spell List"), None)
        rec["spell_list"] = spell_list(spells_section, resolve) if spells_section else []
        for sid in rec["spell_list"]:
            class_spells.setdefault(sid, []).append(cid)
        subs = []
        for c in node.children:
            m = re.match(rf"^{name} Subclass: (.+)$", c.title)
            if not m:
                continue
            sub = {
                "id": make_id("subclass", m.group(1)),
                "name": m.group(1),
                "category": "subclass",
                "class": cid,
                "page": c.page,
                "features": features(c),
            }
            spells = subclass_spells(c, resolve)
            if spells:
                sub["spells"] = spells
            sub["summary"] = summary(c.own_md())
            sub["markdown"] = c.md()
            subs.append(sub)
        rec["subclasses"] = [s["id"] for s in subs]
        subclasses += subs
        rec["summary"] = (
            f"Primary ability {rec['primary_ability_text']}; d{rec['hit_die']} Hit Point Die; saves: "
            + ", ".join(a.title() for a in rec["saving_throws"])
        )
        rec["markdown"] = node.md()
        classes.append(rec)
    return classes, subclasses, class_spells
