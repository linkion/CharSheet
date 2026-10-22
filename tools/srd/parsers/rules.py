"""Rules text: every section of the book not turned into a typed record elsewhere.

Each heading becomes one record holding its own text; parent and children link the tree
back together, so a section can be shown whole or searched piece by piece.
"""

from common import summary
from sections import slug

# Whole chapters that are typed records (classes.json, monsters.json).
TYPED_CHAPTERS = {"Classes", "Monsters A–Z", "Animals"}

# Sections whose children are typed records; the section itself and its intro stay here.
TYPED_CHILDREN = {
    "Background Descriptions", "Species Descriptions",  # backgrounds.json, species.json
    "Origin Feats", "General Feats", "Fighting Style Feats", "Epic Boon Feats",  # feats.json
    "Spell Descriptions",  # spells.json
    "Rules Definitions",  # glossary.json
    "Magic Items A–Z",  # magic_items.json
    "Properties", "Mastery Properties",  # weapon_properties.json, mastery_properties.json
    "Artisan’s Tools", "Other Tools",  # tools.json
    "Adventuring Gear",  # gear.json
    "Lifestyle Expenses",  # services.json
}


def rule_id(node):
    return "rule:" + "/".join(slug(t) for t in node.path)


def parse(root):
    out = []

    def add(node, parent):
        children = [] if node.title in TYPED_CHILDREN else node.children
        out.append({
            "id": rule_id(node),
            "name": node.title,
            "category": "rule",
            "page": node.page,
            "page_end": node.page_end,
            "path": node.path,
            "parent": parent,
            "children": [rule_id(c) for c in children],
            "summary": summary(node.own_md()),
            "markdown": node.own_md(),
        })
        for c in children:
            add(c, rule_id(node))

    for chapter in root.children:
        if chapter.title not in TYPED_CHAPTERS:
            add(chapter, None)
    # Headings that only group typed records ("Origin Feats") have nothing left to say.
    empty = {r["id"] for r in out if not r["markdown"] and not r["children"]}
    for r in out:
        r["children"] = [c for c in r["children"] if c not in empty]
    return [r for r in out if r["id"] not in empty]
