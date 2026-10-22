"""Helpers shared by the category parsers: ids, labelled fields and value parsing."""

import re

from markdown import plain
from sections import slug

ABILITIES = ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"]
ABILITY_ABBR = {"str": "strength", "dex": "dexterity", "con": "constitution", "int": "intelligence", "wis": "wisdom", "cha": "charisma"}
DAMAGE_TYPES = [
    "acid", "bludgeoning", "cold", "fire", "force", "lightning", "necrotic",
    "piercing", "poison", "psychic", "radiant", "slashing", "thunder",
]
CONDITIONS = [
    "blinded", "charmed", "deafened", "exhaustion", "frightened", "grappled", "incapacitated",
    "invisible", "paralyzed", "petrified", "poisoned", "prone", "restrained", "stunned", "unconscious",
]
SKILLS = [
    "acrobatics", "animal_handling", "arcana", "athletics", "deception", "history", "insight",
    "intimidation", "investigation", "medicine", "nature", "perception", "performance",
    "persuasion", "religion", "sleight_of_hand", "stealth", "survival",
]
SCHOOLS = ["abjuration", "conjuration", "divination", "enchantment", "evocation", "illusion", "necromancy", "transmutation"]
SIZES = ["tiny", "small", "medium", "large", "huge", "gargantuan"]
CREATURE_TYPES = [
    "aberration", "beast", "celestial", "construct", "dragon", "elemental", "fey", "fiend",
    "giant", "humanoid", "monstrosity", "ooze", "plant", "undead",
]
CLASSES = ["barbarian", "bard", "cleric", "druid", "fighter", "monk", "paladin", "ranger", "rogue", "sorcerer", "warlock", "wizard"]


def key(text):
    """snake_case enum value: "Sleight of Hand" -> "sleight_of_hand"."""
    return slug(text).replace("-", "_")


def make_id(category, name):
    return f"{category}:{slug(name)}"


def summary(md):
    """The first sentence of some Markdown, for search results."""
    text = plain(md)
    for line in text.split("\n"):
        line = line.strip().lstrip("|>-# ").strip()
        if not line or line.startswith("---"):
            continue
        m = re.match(r"(.+?[.!?])(\s|$)", line)
        out = m.group(1) if m else line
        return out if len(out) <= 300 else out[:297].rstrip() + "…"
    return ""


def fields(md):
    """Splits "**Label:** value **Other:** value" text into {label: value}.

    Works across paragraphs and table cells, so it reads background and tool blocks alike.
    """
    text = md.replace("|", " ")
    parts = re.split(r"\*\*([^*]+?):\*\*", text)
    out = {}
    for label, value in zip(parts[1::2], parts[2::2]):
        out[label.strip()] = re.sub(r"\s+", " ", value).strip()
    return out


def split_list(text):
    """"A, B, and C" / "A or B" -> [A, B, C]."""
    text = re.sub(r"\s*\(see [^)]*\)", "", text)
    items = re.split(r",\s*(?:and\s+|or\s+)?|\s+(?:and|or)\s+", text)
    return [i.strip().rstrip(".") for i in items if i.strip()]


def cost(text):
    """"25 GP" -> {"amount": 25, "unit": "gp"}; "—" -> None."""
    m = re.search(r"([\d,]+(?:\.\d+)?)\s*(CP|SP|EP|GP|PP)\b", text or "", re.I)
    if not m:
        return None
    amount = float(m.group(1).replace(",", ""))
    return {"amount": int(amount) if amount.is_integer() else amount, "unit": m.group(2).lower()}


FRACTIONS = {"½": 0.5, "¼": 0.25, "¾": 0.75}


def number(text):
    """"1½" -> 1.5, "1/4" -> 0.25, "1,320" -> 1320; None when there's no number."""
    m = re.search(r"(\d[\d,]*(?:\.\d+)?)?([½¼¾]|/\d+)?", (text or "").strip())
    if not m or not (m.group(1) or m.group(2)):
        return None
    whole = float((m.group(1) or "0").replace(",", ""))
    frac = m.group(2)
    if frac in FRACTIONS:
        whole += FRACTIONS[frac]
    elif frac:
        whole /= float(frac[1:])
    return int(whole) if whole.is_integer() else whole


def weight(text):
    """"1/4 lb." -> 0.25, "1½ lb." -> 1.5; "—" -> None."""
    m = re.search(r"([\d,./½¼¾]+)\s*lb", text or "")
    return number(m.group(1)) if m else None


DICE = re.compile(r"\b(\d*)d(\d+)(?:\s*([+−-])\s*(\d+))?\b")


def dice(text):
    """"2d6 + 5" -> "2d6+5", normalised for the app's dice roller."""
    m = DICE.search(text or "")
    if not m:
        return None
    out = f"{m.group(1) or 1}d{m.group(2)}"
    if m.group(4):
        out += ("-" if m.group(3) in "−-" else "+") + m.group(4)
    return out


def signed(text):
    """"+5" / "−1" -> 5 / -1."""
    text = text.strip().replace("−", "-").replace("–", "-")
    return int(text)


_SCORE = r"\** (\d+) ([+−-]?\d+) ([+−-]?\d+)"
ABILITY_LINES = re.compile(
    rf"^\**Str{_SCORE} \**Dex{_SCORE} \**Con{_SCORE}\n\n\**Int{_SCORE} \**Wis{_SCORE} \**Cha{_SCORE}$", re.M
)


def ability_tables(md):
    """Rewrites a stat block's two "Str 21 +5 +5 Dex …" lines as a table."""

    def table(m):
        rows = ["| | Score | Mod | Save |", "|---|---|---|---|"]
        for i, a in enumerate(["Str", "Dex", "Con", "Int", "Wis", "Cha"]):
            score, mod, save = m.group(3 * i + 1), m.group(3 * i + 2), m.group(3 * i + 3)
            rows.append(f"| {a} | {score} | {mod} | {save} |")
        return "\n".join(rows)

    return ABILITY_LINES.sub(table, md)


def table_rows(token):
    """A table token's rows as dicts keyed by plain header text."""
    header = [plain(h) for h in token.header]
    return [dict(zip(header, r)) for r in token.rows]
