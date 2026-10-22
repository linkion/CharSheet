"""Feats."""

import re

from common import key, make_id, summary
from markdown import plain

HEADER = re.compile(r"^(Origin|General|Fighting Style|Epic Boon) Feat(?: \(Prerequisite: (.*)\))?$")


def parse(chapter):
    out = []
    for group in chapter.child("Feat Descriptions").children:
        if not group.title.endswith("Feats"):
            continue
        for node in group.children:
            m = HEADER.match(plain(node.tokens[0].md))
            if not m:
                raise ValueError(f"feat {node.title!r}: {node.tokens[0].md!r}")
            benefits = []
            for t in node.tokens[1:]:
                b = re.match(r"^\*\*\*(.+?)\.\*\*\*\s*(.*)$", t.md, re.S) if t.kind == "para" else None
                if b and b.group(1) != "Repeatable":
                    benefits.append({"name": b.group(1), "markdown": b.group(2)})
                elif benefits and not b:
                    benefits[-1]["markdown"] += "\n\n" + t.to_md()
            rec = {
                "id": make_id("feat", node.title),
                "name": node.title,
                "category": "feat",
                "page": node.page,
                "feat_category": key(m.group(1)),
                "repeatable": "***Repeatable.***" in node.own_md(),
                "benefits": benefits,
            }
            if m.group(2):
                rec["prerequisite"] = m.group(2)
                level = re.search(r"Level (\d+)\+", m.group(2))
                if level:
                    rec["prerequisite_level"] = int(level.group(1))
            body = "\n\n".join(t.to_md() for t in node.tokens[1:])
            rec["summary"] = summary(body)
            if rec["summary"] == "You gain the following benefits." and benefits:
                rec["summary"] = "Benefits: " + ", ".join(b["name"] for b in benefits) + "."
            rec["markdown"] = node.md()
            out.append(rec)
    return out
