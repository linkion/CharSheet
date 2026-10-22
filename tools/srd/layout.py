"""Turns the PDF into styled lines in reading order.

mutool's structured text gives every character with its font, size, colour and position.
This module groups those into lines of styled spans, drops the running footer and orders
each page's lines the way a reader would: full-width rows top to bottom, with the two text
columns between them read left column first.
"""

import io
import re
import subprocess
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field

PAGE_WIDTH = 594
COLUMN_SPLIT = PAGE_WIDTH / 2
FOOTER_Y = 740

RED = "#8c2220"


@dataclass
class Span:
    text: str
    font: str
    size: float
    color: str
    x0: float
    x1: float

    @property
    def bold(self):
        return any(w in self.font for w in ("Bold", "SemiBold"))

    @property
    def italic(self):
        return "Italic" in self.font

    @property
    def sans(self):
        return self.font.startswith(("GillSans", "Optima", "ScalaSans"))


@dataclass
class Line:
    page: int
    x0: float
    y0: float
    x1: float
    y1: float
    spans: list = field(default_factory=list)
    base: float = 0.0

    @property
    def text(self):
        return "".join(s.text for s in self.spans)

    @property
    def first(self):
        return next((s for s in self.spans if s.text.strip()), self.spans[0])

    @property
    def size(self):
        return self.first.size

    @property
    def baseline(self):
        return self.base or self.y1

    def __repr__(self):
        f = self.first
        return f"<Line p{self.page} ({self.x0:.0f},{self.y0:.0f}) {f.font}/{f.size}/{f.color} {self.text!r}>"


@dataclass
class Block:
    x0: float
    y0: float
    x1: float
    y1: float
    lines: list = field(default_factory=list)

    @property
    def full_width(self):
        return self.x0 < COLUMN_SPLIT - 10 and self.x1 > COLUMN_SPLIT + 10


def _spans(line_el):
    """Splits a line into spans at every font or colour change, and at wide gaps.

    A wide gap between characters (a table column boundary) becomes a span holding a tab, so
    table rows that mutool merged into one line can be split into cells again.
    """
    spans = []
    for font_el in line_el.iter("font"):
        name, size = font_el.get("name"), float(font_el.get("size"))
        for ch in font_el.iter("char"):
            c = ch.get("c")
            quad = [float(v) for v in ch.get("quad").split()]
            x0, x1 = quad[0], quad[2]
            color = ch.get("color")
            prev = spans[-1] if spans else None
            if prev and x0 - prev.x1 > size * 1.2 and prev.text.strip() and c.strip():
                spans.append(Span("\t", name, size, color, prev.x1, x0))
                prev = spans[-1]
            if prev and prev.font == name and prev.size == size and prev.color == color and prev.text != "\t":
                prev.text += c
                prev.x1 = max(prev.x1, x1)
            else:
                spans.append(Span(c, name, size, color, x0, x1))
    return spans


def read_pages(pdf, first=1, last=None):
    """Yields (page number, [Block]) for every page, in mutool's order."""
    args = ["mutool", "draw", "-q", "-F", "stext", "-o", "-", pdf]
    if first > 1 or last:
        args.append(f"{first}-{last or 'N'}")
    xml = subprocess.run(args, capture_output=True, check=True).stdout
    # A few glyphs map to control characters, which XML doesn't allow.
    xml = re.sub(rb"&#x(?:[0-8bcef]|1[0-9a-f]);", b"", xml, flags=re.I)
    page_no = None
    blocks = []
    for event, el in ET.iterparse(io.BytesIO(xml), events=("start", "end")):
        if event == "start" and el.tag == "page":
            page_no = int(el.get("id").removeprefix("page"))
            blocks = []
        elif event == "start" and el.tag == "block":
            x0, y0, x1, y1 = (float(v) for v in el.get("bbox").split())
            blocks.append(Block(x0, y0, x1, y1))
        elif event == "end" and el.tag == "line":
            x0, y0, x1, y1 = (float(v) for v in el.get("bbox").split())
            spans = _spans(el)
            if spans and "".join(s.text for s in spans).strip():
                base = float(next(el.iter("char")).get("y"))
                blocks[-1].lines.append(Line(page_no, x0, y0, x1, y1, spans, base))
            el.clear()
        elif event == "end" and el.tag == "page":
            yield page_no, [b for b in blocks if b.lines]
            el.clear()


def _rows(lines):
    """Joins lines that share a baseline into one line.

    Spans are put back in x order; a wide gap between them (a table column boundary)
    becomes a tab span, a narrow one a space.
    """
    groups = []
    for line in sorted(lines, key=lambda l: l.baseline):
        if groups and abs(groups[-1][0].baseline - line.baseline) < 2:
            groups[-1].append(line)
        else:
            groups.append([line])
    out = []
    for group in groups:
        if len(group) == 1:
            out.append(group[0])
            continue
        spans = sorted((s for l in group for s in l.spans if s.text != "\t"), key=lambda s: s.x0)
        # Re-split each line's own spans at its tabs so cells stay apart.
        cells = []
        for line in sorted(group, key=lambda l: l.x0):
            cell = []
            for s in line.spans:
                if s.text == "\t":
                    cells.append(cell)
                    cell = []
                else:
                    cell.append(s)
            cells.append(cell)
        cells = sorted((c for c in cells if c), key=lambda c: c[0].x0)
        merged = list(cells[0])
        for cell in cells[1:]:
            prev = merged[-1]
            gap = cell[0].x0 - prev.x1
            if gap < cell[0].size * 0.6:
                if not (prev.text.endswith(" ") or cell[0].text.startswith(" ")):
                    merged.append(Span(" ", prev.font, prev.size, prev.color, prev.x1, cell[0].x0))
            else:
                merged.append(Span("\t", prev.font, prev.size, prev.color, prev.x1, cell[0].x0))
            merged += cell
        first = group[0]
        out.append(Line(
            first.page,
            min(l.x0 for l in group),
            min(l.y0 for l in group),
            max(l.x1 for l in group),
            max(l.y1 for l in group),
            merged,
            first.base,
        ))
    return out


def _table_like(block):
    """True when every line of a block is in a table cell font."""
    return all(l.first.font.startswith("GillSans") and l.first.size < 10 for l in block.lines)


def is_caption(line):
    """A table's title: black GillSans SemiBold just above body size."""
    f = line.first
    return f.font == "GillSans-SemiBold" and 10 <= f.size <= 11 and f.color != RED


def order(blocks):
    """Puts a page's lines in reading order.

    mutool groups text into blocks; a block that crosses the middle of the page is full
    width (a heading or a wide table row). Full-width blocks cut the page into bands, and
    inside each band the left column is read before the right one.
    """
    blocks = [b for b in blocks if b.y0 < FOOTER_Y]
    full = sorted((b for b in blocks if b.full_width), key=lambda b: b.y0)
    cols = [b for b in blocks if not b.full_width]
    # A full-width table's caption sits in a column just above it; read it with the table.
    for b in full:
        for c in cols:
            if c.lines and is_caption(c.lines[-1]) and 0 <= b.y0 - c.y1 < 20 and len(c.lines) == 1:
                b.lines = c.lines + b.lines
                b.y0 = c.y0
                cols.remove(c)
                break
    # Overlapping full-width blocks (cells of one table row) are read together.
    clusters = []
    for b in full:
        if clusters and b.y0 < clusters[-1].y1 - 1:
            c = clusters[-1]
            c.lines += b.lines
            c.x0, c.x1, c.y1 = min(c.x0, b.x0), max(c.x1, b.x1), max(c.y1, b.y1)
        else:
            clusters.append(Block(b.x0, b.y0, b.x1, b.y1, list(b.lines)))
    # Column blocks wholly inside a full-width table's rows are part of it, and so are
    # column blocks that fill the gap between two rows of the same table.
    for c in clusters:
        for b in list(cols):
            if b.y0 >= c.y0 - 1 and b.y1 <= c.y1 + 1:
                c.lines += b.lines
                cols.remove(b)
    merged = []
    for c in clusters:
        prev = merged[-1] if merged else None
        between = prev and [b for b in cols if b.y0 >= prev.y1 - 1 and b.y1 <= c.y0 + 1]
        if prev and c.y0 - prev.y1 < 12 and all(_table_like(b) for b in between) and (
            between or _table_like(prev)
        ):
            prev.lines += c.lines + [l for b in between for l in b.lines]
            prev.y1 = c.y1
            for b in between:
                cols.remove(b)
        else:
            merged.append(c)
    out = []
    top = -1
    bands = []
    for b in merged:
        bands.append((top, b.y0, None))
        bands.append((b.y0, b.y1, b))
        top = b.y1
    bands.append((top, 10_000, None))
    used = set()
    for y0, y1, block in bands:
        if block is not None:
            out += _rows(block.lines)
            continue
        inside = [b for b in cols if id(b) not in used and (b.y0 + b.y1) / 2 < y1]
        used.update(id(b) for b in inside)
        for side in (lambda b: b.x0 < COLUMN_SPLIT, lambda b: b.x0 >= COLUMN_SPLIT):
            lines = [l for b in inside if side(b) for l in b.lines]
            out += _rows(lines)
    return out


def document(pdf, first=1, last=None):
    """Every page's lines in reading order, as one list."""
    out = []
    for _, lines in read_pages(pdf, first, last):
        out += order(lines)
    return out
