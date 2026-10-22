"""Equipment: weapons, armor, tools, gear, mounts and vehicles, and services."""

import re

from common import ABILITIES, cost, dice, fields, key, make_id, number, summary, weight
from markdown import plain

PRICED = re.compile(r"^(.*?) \(([^()]*(?:\([^()]*\))?[^()]*)\)$")


def rows(table):
    """A table's rows as (group, {header: cell}) pairs.

    Tables printed as two side-by-side halves (Item | Cost | Item | Cost) are unfolded, and
    an italic row with no other cells ("*Simple Melee Weapons*") labels the rows under it.
    """
    header = [plain(h) for h in table.header]
    body = [list(r) for r in table.rows]
    half = len(header) // 2
    if len(header) % 2 == 0 and half > 1 and header[:half] == header[half:]:
        # Unfold each part (column or page) of the table separately to keep reading order.
        cuts = [0] + table.breaks + [len(body)]
        parts = [body[a:b] for a, b in zip(cuts, cuts[1:])]
        body = [row for p in parts for row in [r[:half] for r in p] + [r[half:] for r in p]]
        header = header[:half]
    out, group = [], None
    for r in body:
        cells = [c.strip() for c in r]
        if not any(cells):
            continue
        if re.fullmatch(r"\*[^*]+\*", cells[0]) and not any(cells[1:]):
            group = plain(cells[0])
            continue
        out.append((group, dict(zip(header, cells))))
    return out


def find_table(section, caption):
    for s in section.walk():
        for t in s.tokens:
            if t.kind == "table" and t.caption == caption:
                return t
    raise KeyError(caption)


def entry(category, name, page, md, **fields_):
    rec = {"id": make_id(category, name), "name": name, "category": category, "page": page}
    rec.update(fields_)
    rec["summary"] = summary(md) if md else ""
    rec["markdown"] = md
    return rec


# --- weapons ---------------------------------------------------------------------------

PROPERTY = re.compile(r"^([A-Za-z-]+(?: [A-Za-z-]+)?)(?: \((.*)\))?$")


def weapon_properties_list(text):
    """"Ammunition (Range 80/320; Bolt), Loading" -> [{id, name, ...params}]."""
    out = []
    if text.strip() == "—":
        return out
    for part in re.split(r",\s*(?![^()]*\))", text):
        m = PROPERTY.match(part.strip())
        name, params = m.group(1), m.group(2)
        prop = {"id": make_id("weapon_property", name), "name": name}
        if params:
            r = re.search(r"Range (\d+)/(\d+)", params)
            if r:
                prop["range"] = {"normal": int(r.group(1)), "long": int(r.group(2))}
            ammo = re.search(r"; (\w+)$", params)
            if ammo:
                prop["ammunition"] = ammo.group(1).lower()
            d = dice(params)
            if d and name == "Versatile":
                prop["damage"] = d
            if not (r or d):
                prop["note"] = params
        out.append(prop)
    return out


def weapons(chapter):
    section = chapter.child("Weapons")
    out = []
    for group, r in rows(find_table(section, "Weapons")):
        cat, kind = group.split()[:2]  # "Simple Melee Weapons"
        dmg = plain(r["Damage"])
        m = re.match(r"^(\S+) (\w+)$", dmg)
        props = weapon_properties_list(plain(r["Properties"]))
        name = plain(r["Name"])
        rec = {
            "id": make_id("weapon", name),
            "name": name,
            "category": "weapon",
            "page": find_table(section, "Weapons").page,
            "weapon_category": cat.lower(),
            "ranged": kind == "Ranged",
            "damage": {"dice": dice(m.group(1)) or m.group(1), "type": m.group(2).lower()},
            "properties": props,
            "mastery": make_id("mastery_property", plain(r["Mastery"])),
            "weight_lb": weight(r["Weight"]),
            "cost": cost(r["Cost"]),
        }
        rec["summary"] = f"{group[:-1]}: {dmg}; {plain(r['Properties'])}; mastery {plain(r['Mastery'])}"
        rec["markdown"] = "\n".join(f"**{k}:** {v}" for k, v in r.items())
        out.append(rec)
    return out


def property_entries(section, category):
    return [entry(category, c.title, c.page, c.md()) for c in section.children]


# --- armor -----------------------------------------------------------------------------


def armor(chapter):
    section = chapter.child("Armor")
    table = find_table(section, "Armor")
    out = []
    for group, r in rows(table):
        name = plain(r["Armor"])
        ac_text = plain(r["Armor Class (AC)"])
        ac = {"text": ac_text}
        m = re.match(r"^\+?(\d+)", ac_text)
        if ac_text.startswith("+"):
            ac["bonus"] = int(m.group(1))
        else:
            ac["base"] = int(m.group(1))
            if "Dex" in ac_text:
                cap = re.search(r"max (\d+)", ac_text)
                ac["dex_cap"] = int(cap.group(1)) if cap else None
            else:
                ac["dex_cap"] = 0
        strength = re.search(r"(\d+)", plain(r["Strength"]))
        g = re.match(r"^(\w+)(?: Armor)? \((.*)\)$", group)
        rec = {
            "id": make_id("armor", name),
            "name": name,
            "category": "armor",
            "page": table.page,
            "armor_category": g.group(1).lower(),
            "ac": ac,
            "strength_required": int(strength.group(1)) if strength else None,
            "stealth_disadvantage": plain(r["Stealth"]) == "Disadvantage",
            "don_doff": g.group(2),
            "weight_lb": weight(r["Weight"]),
            "cost": cost(r["Cost"]),
        }
        rec["summary"] = f"{group}: AC {ac_text}"
        rec["markdown"] = "\n".join(f"**{k}:** {v}" for k, v in r.items())
        out.append(rec)
    return out


# --- tools -----------------------------------------------------------------------------


def priced(title):
    """"Alchemist’s Supplies (50 GP)" -> ("Alchemist’s Supplies", "50 GP")."""
    m = PRICED.match(title)
    return (m.group(1), m.group(2)) if m else (title, None)


def tools(chapter):
    out = []
    section = chapter.child("Tools")
    for group in ("Artisan’s Tools", "Other Tools"):
        for node in section.child(group).children:
            name, price = priced(node.title)
            # The PDF sets a tool's details as a two-column box, sometimes split over two
            # columns of the page; flatten it and read the labelled fields.
            text = " ".join(t.to_md() for t in node.tokens)
            text = re.sub(r"\|?[ \t]*-{3}[-| ]*", " ", text).replace("|", " ")
            text = re.sub(r"\s+", " ", text)
            text = re.sub(r"(\w)- ([a-z])", r"\1\2", text)  # a word hyphenated across the split
            f = fields(text)
            # When the box continues in the next column, the rest of the field before
            # Weight lands after it: "Weight: Varies cimer (25 GP, …)".
            w = re.match(r"^(—|Varies|[\d½¼¾/.,]+ lb\.)\s*(.*)$", f.get("Weight", ""))
            if w and w.group(2):
                keys = list(f)
                before = keys[keys.index("Weight") - 1]
                f[before] = f[before][:-1] + w.group(2) if f[before].endswith("-") else f"{f[before]} {w.group(2)}"
                f["Weight"] = w.group(1)
            order = ["Ability", "Utilize", "Craft", "Variants", "Weight"]
            md = "\n\n".join(f"**{k}:** {f[k]}" for k in sorted(f, key=lambda k: order.index(k) if k in order else 99))
            rec = entry(
                "tool", name, node.page, md,
                tool_category="artisans_tools" if group.startswith("Artisan") else "other",
                ability=f.get("Ability", "").lower() or None,
                utilize=[u.strip() for u in re.split(r",\s*or\s+|,\s*(?=[A-Z])", plain(f.get("Utilize", ""))) if u.strip()],
                craft=[c.strip() for c in re.split(r",\s*(?![^()]*\))", plain(f.get("Craft", ""))) if c.strip()],
                weight_lb=weight(f.get("Weight")),
                cost=cost(price),
            )
            if "Variants" in f:
                rec["variants"] = [v.strip() for v in re.split(r",\s*(?![^()]*\))", plain(f["Variants"]).rstrip("."))]
            assert rec["ability"] in ABILITIES, (name, rec["ability"])
            rec["summary"] = f"{rec['ability'].title()} tool. Utilize: {plain(f.get('Utilize', ''))}"
            out.append(rec)
    return out


# --- adventuring gear ------------------------------------------------------------------


def gear(chapter):
    section = chapter.child("Adventuring Gear")
    table = {plain(r["Item"]): r for _, r in rows(find_table(section, "Adventuring Gear"))}
    out = []
    for node in section.children:
        name, price = priced(node.title)
        row = table.pop(name, None)
        variants = []
        # The Adventuring Gear table itself is printed inside the Ammunition entry.
        tokens = [t for t in node.tokens if not (t.kind == "table" and t.caption == "Adventuring Gear")]
        md = "\n\n".join(t.to_md() for t in tokens)
        for t in tokens:
            if t.kind == "table":
                first = plain(t.header[0])
                for _, r in rows(t):
                    v = {"name": plain(r[first])}
                    for k, val in r.items():
                        if k == first:
                            continue
                        if k == "Weight":
                            v["weight_lb"] = weight(val)
                        elif k == "Cost":
                            v["cost"] = cost(val)
                        else:
                            v[key(k)] = plain(val)
                    variants.append(v)
        rec = entry(
            "gear", name, node.page, md,
            weight_lb=weight(row["Weight"]) if row else None,
            cost=cost(row["Cost"]) if row else cost(price),
        )
        if price and not rec["cost"]:
            rec["cost_text"] = price
        if variants:
            rec["variants"] = variants
        out.append(rec)
    # Table rows without a description of their own are variants of one that has it,
    # such as "Spell Scroll (Cantrip)".
    by_name = {r["name"]: r for r in out}
    for name, r in table.items():
        base = by_name.get(re.sub(r"\s*\(.*\)$", "", name))
        if base is None:
            raise KeyError(f"gear table row {name!r} has no description")
        base.setdefault("variants", []).append({"name": name, "weight_lb": weight(r["Weight"]), "cost": cost(r["Cost"])})
        if base["cost"] is None or base["cost"] == cost(r["Cost"]):
            base["cost"] = None
    return out


# --- mounts and vehicles ---------------------------------------------------------------


def mounts_vehicles(chapter):
    section = chapter.child("Mounts and Vehicles")
    out = []
    for _, r in rows(find_table(section, "Mounts and Other Animals")):
        name = plain(r["Item"])
        cap = weight(r["Carrying Capacity"])
        out.append(entry("mount", name, section.page, "", vehicle_type="mount", carrying_capacity_lb=cap, cost=cost(r["Cost"])))
    ended = set()
    for group, r in rows(find_table(section, "Tack, Harness, and Drawn Vehicles")):
        name = plain(r["Item"])
        # The table is alphabetical; the indented Saddle rows end where the order resumes.
        if group and group not in ended and name.lower() < group.lower():
            name = f"{group}, {name}"
        elif group:
            ended.add(group)
        out.append(entry("vehicle", name, section.page, "", vehicle_type="tack_or_drawn", weight_lb=weight(r["Weight"]), cost=cost(r["Cost"])))
    for _, r in rows(find_table(section, "Airborne and Waterborne Vehicles")):
        name = plain(r["Ship"])

        def num(k):
            return number(plain(r[k]))

        out.append(entry(
            "vehicle", name, section.page, "",
            vehicle_type="ship",
            speed_mph=number(plain(r["Speed"])),
            crew=num("Crew"), passengers=num("Passengers"), cargo_tons=num("Cargo (Tons)"),
            ac=num("AC"), hp=num("HP"), damage_threshold=num("Damage Threshold"), cost=cost(r["Cost"]),
        ))
    for rec in out:
        rec["summary"] = ", ".join(f"{k.replace('_', ' ')} {v}" for k, v in rec.items() if k in ("carrying_capacity_lb", "weight_lb", "speed_mph") and v)
        rec["markdown"] = rec["markdown"] or f"**{rec['name']}**"
    return out


# --- services --------------------------------------------------------------------------


def services(chapter):
    out = []
    life = chapter.child("Lifestyle Expenses")
    for node in life.children:
        name, price = priced(node.title)
        price_ = {"amount": 0, "unit": "gp"} if price == "Free" else cost(price)
        out.append(entry("service", f"{name} Lifestyle", node.page, node.md(), service_type="lifestyle", cost=price_, per="day"))
    food = chapter.child("Food, Drink, and Lodging")
    for group, r in rows(find_table(food, "Food, Drink, and Lodging")):
        item = plain(r["Item"])
        name = f"{item} ({group})" if group else item
        out.append(entry("service", name, food.page, "", service_type=key(group or "food_and_drink"), cost=cost(r["Cost"])))
    hire = chapter.child("Hirelings")
    for _, r in rows(find_table(hire, "Hirelings")):
        c = plain(r["Cost"])
        out.append(entry("service", plain(r["Service"]), hire.page, "", service_type="hireling", cost=cost(c), per=c.split(" per ")[-1]))
    casting = chapter.child("Spellcasting")
    for _, r in rows(find_table(casting, "Spellcasting Services")):
        level = plain(r["Spell Level"])
        out.append(entry(
            "service", f"Spellcasting ({'Cantrip' if level == 'Cantrip' else 'Level ' + level})", casting.page, "",
            service_type="spellcasting", availability=plain(r["Availability"]), cost=cost(r["Cost"]),
        ))
    for rec in out:
        if not rec["markdown"]:
            rec["markdown"] = f"**{rec['name']}**: {rec['cost']['amount']} {rec['cost']['unit'].upper()}" if rec["cost"] else ""
            rec["summary"] = plain(rec["markdown"])
    return out


def parse(chapter):
    weapon_section = chapter.child("Weapons")
    return {
        "weapons": weapons(chapter),
        "weapon_properties": property_entries(weapon_section.child("Properties"), "weapon_property"),
        "mastery_properties": property_entries(weapon_section.child("Mastery Properties"), "mastery_property"),
        "armor": armor(chapter),
        "tools": tools(chapter),
        "gear": gear(chapter),
        "mounts_vehicles": mounts_vehicles(chapter),
        "services": services(chapter),
    }
