"""Magic items: "Magic Items A–Z"."""

import re

from common import CLASSES, dice, key, make_id, summary
from markdown import plain

TYPES = ["Wondrous Item", "Weapon", "Armor", "Potion", "Ring", "Wand", "Staff", "Rod", "Scroll"]
RARITY = r"(Common|Uncommon|Rare|Very Rare|Legendary|Artifact)"


def header(text):
    """"Armor (Plate Armor), Legendary (Requires Attunement by a Paladin)"."""
    out = {}
    att = re.search(r"\s*\(Requires Attunement(?: by (an? )?([^)]*))?\)$", text)
    out["attunement"] = {"required": bool(att)}
    if att:
        text = text[: att.start()]
        if att.group(2):
            by = att.group(2)
            out["attunement"]["by"] = by
            classes = [make_id("class", c) for c in re.findall(r"\w+", by) if c.lower() in CLASSES]
            if classes:
                out["attunement"]["classes"] = classes
            elif by.lower() == "spellcaster":
                out["attunement"]["spellcaster"] = True
    m = re.match(rf"^({'|'.join(TYPES)})(?: \(([^)]*)\))?, (.*)$", text)
    if not m:
        raise ValueError(f"item header {text!r}")
    out["item_type"] = key(m.group(1))
    if m.group(2):
        out["subtype"] = m.group(2)
    rarity = m.group(3)
    variants = re.findall(rf"{RARITY} \(([^)]*)\)", rarity)
    if re.fullmatch(RARITY, rarity):
        out["rarity"] = key(rarity)
    elif rarity == "Rarity Varies":
        out["rarity"] = "varies"
    elif variants:
        out["rarity"] = "varies"
        out["rarity_by_variant"] = [{"variant": v, "rarity": key(r)} for r, v in variants]
    else:
        raise ValueError(f"rarity {rarity!r}")
    return out


def aliases(name):
    """"Armor, +1, +2, or +3" -> ["+1 Armor", "+2 Armor", "+3 Armor"]."""
    m = re.match(r"^(.*?), (\+\d(?:,? (?:or )?\+\d)*)$", name)
    if m:
        return [f"{b} {m.group(1)}" for b in re.findall(r"\+\d", m.group(2))]
    m = re.match(r"^(.*?), (.*)$", name)
    if m:
        return [f"{m.group(2)} {m.group(1)}"]
    return []


def parse(section):
    out = []
    for node in section.children:
        if not node.tokens:
            raise ValueError(f"item {node.title!r} has no text")
        head = header(plain(node.tokens[0].md))
        body = node.tokens[1:]
        body_md = "\n\n".join(t.to_md() for t in body)
        text = plain(body_md)
        rec = {
            "id": make_id("magic_item", node.title),
            "name": node.title,
            "category": "magic_item",
            "page": node.page,
            "aliases": aliases(node.title),
        }
        rec.update(head)
        charges = re.search(r"has (\d+) charges", text)
        if charges:
            c = {"max": int(charges.group(1))}
            regain = re.search(r"regains (all|\d+d\d+(?: \+ \d+)?|\d+) expended charges? (daily at dawn|[^.]*)", text)
            if regain:
                c["regain"] = regain.group(1) if regain.group(1) == "all" else dice(regain.group(1)) or int(regain.group(1))
                c["regain_when"] = "dawn" if "dawn" in regain.group(2) else regain.group(2).strip()
            rec["charges"] = c
        rec["cursed"] = bool(re.search(r"\*\*\*Curse\.\*\*\*|\bcursed\b", body_md))
        rec["summary"] = summary(body_md)
        rec["markdown"] = node.md()
        out.append(rec)
    return out
