"""Builds the book's heading tree from the token stream."""

import re
import unicodedata
from dataclasses import dataclass, field


def slug(text):
    text = unicodedata.normalize("NFKD", text).encode("ascii", "ignore").decode()
    text = text.lower().replace("&", " and ").replace("+", " plus ")
    return re.sub(r"[^a-z0-9]+", "-", text).strip("-")


@dataclass
class Section:
    title: str
    level: int
    page: int
    tokens: list = field(default_factory=list)
    children: list = field(default_factory=list)
    parent: "Section" = None
    page_end: int = 0
    role: str = ""

    def __repr__(self):
        return f"<Section L{self.level} p{self.page} {self.title!r} {len(self.children)} children>"

    @property
    def path(self):
        out, node = [], self
        while node and node.level > 0:
            out.append(node.title)
            node = node.parent
        return out[::-1]

    def walk(self):
        yield self
        for c in self.children:
            yield from c.walk()

    def find(self, title, level=None):
        for s in self.walk():
            if s.title == title and (level is None or s.level == level):
                return s
        raise KeyError(title)

    def child(self, title):
        for c in self.children:
            if c.title == title:
                return c
        raise KeyError(f"{title} under {self.title}")

    def own_md(self):
        return "\n\n".join(t.to_md() for t in self.tokens)

    def md(self, include_heading=False, depth=0):
        """This section's text and all its subsections', headings relative to it."""
        parts = []
        if include_heading:
            parts.append("#" * min(depth + 1, 6) + " " + self.title)
        if self.tokens:
            parts.append(self.own_md())
        for c in self.children:
            parts.append(c.md(True, depth + 1))
        return "\n\n".join(p for p in parts if p)


def _mentions(section, caption):
    text = " ".join(t.md for t in section.tokens if t.kind == "para").lower()
    return f"{caption.lower()} table" in text.replace("*", "")


def _move_tables(root):
    """Puts each captioned table with the entry that refers to it.

    Full-width tables are read after both columns above them, so one can land in whatever
    entry comes next. When the entry holding a table never names it but a nearby entry
    says "…the <caption> table", the table moves there.
    """
    nodes = list(root.walk())
    for node in nodes:
        for t in list(node.tokens):
            if t.kind != "table" or not t.caption or _mentions(node, t.caption):
                continue
            owners = [
                n for n in nodes
                if n is not node and abs(n.page - t.page) <= 1 and _mentions(n, t.caption)
            ]
            if len(owners) == 1:
                node.tokens.remove(t)
                owners[0].tokens.append(t)


def build(tokens):
    root = Section("SRD 5.2.1", 0, 1)
    stack = [root]
    for t in tokens:
        if t.kind == "heading":
            level = t.level
            if t.role == "statblock":
                # Stat blocks also appear inside spells and magic items: nest them under
                # whatever entry they follow, but never under another stat block.
                while stack[-1].role in ("statblock", "stat_section"):
                    stack.pop()
                level = max(3, stack[-1].level + 1)
            elif t.role == "stat_section":
                while stack[-1].role == "stat_section":
                    stack.pop()
                level = stack[-1].level + 1 if stack[-1].role == "statblock" else t.level
            while stack[-1].level >= level:
                stack.pop()
            node = Section(t.md, level, t.page, parent=stack[-1], role=t.role)
            stack[-1].children.append(node)
            stack.append(node)
        else:
            stack[-1].tokens.append(t)
    _move_tables(root)
    # Fill page_end bottom-up.
    for s in reversed(list(root.walk())):
        pages = [s.page] + [t.page for t in s.tokens] + [c.page_end for c in s.children]
        s.page_end = max(pages)
    return root
