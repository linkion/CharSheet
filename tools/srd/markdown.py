"""Turns styled lines into a stream of tokens: headings, paragraphs, tables and sidebars.

Paragraph and table text is Markdown: bold and italic spans are kept, words hyphenated at a
line break are rejoined, and table rows are rebuilt from the cells' x positions.
"""

import re
from collections import Counter
from dataclasses import dataclass, field

from layout import COLUMN_SPLIT, RED

DARK_RED = "#540000"


@dataclass
class Token:
    kind: str  # heading, para, table, sidebar
    page: int
    level: int = 0  # headings: 1 chapter … 5 stat block section
    md: str = ""  # headings and paragraphs
    caption: str = ""  # tables and sidebars
    header: list = field(default_factory=list)  # tables
    rows: list = field(default_factory=list)  # tables
    body: list = field(default_factory=list)  # sidebars: their own paragraphs and tables
    breaks: list = field(default_factory=list)  # tables: row indexes where a new column or page part starts
    role: str = ""  # headings: "statblock" for a stat block's name, "stat_section" for Traits etc.

    @property
    def text(self):
        return plain(self.md)

    def to_md(self):
        if self.kind == "heading":
            return "#" * min(self.level + 1, 6) + " " + self.md
        if self.kind == "para":
            return self.md
        if self.kind == "table":
            return table_md(self)
        if self.kind == "sidebar":
            inner = "\n\n".join(t.to_md() for t in self.body)
            quoted = "\n".join("> " + l if l else ">" for l in inner.split("\n"))
            return f"> **{self.caption}**\n>\n{quoted}"
        raise ValueError(self.kind)


def tidy(md):
    """Merges emphasis runs that meet, e.g. "**Saving Throw** **Proficiencies**"."""
    md = re.sub(r"(?<=[^*\s])\*\*\*(\s*)\*\*\*(?=[^*\s])", r"\1", md)
    md = re.sub(r"(?<=[^*\s])\*\*(\s*)\*\*(?=[^*\s])", r"\1", md)
    md = re.sub(r"(?<=[^*\s])\*(\s+)\*(?=[^*\s])", r"\1", md)
    return md


def plain(md):
    """Strips Markdown emphasis, keeping literal (escaped) asterisks."""
    return re.sub(r"(?<!\\)\*+", "", md).replace("\\*", "*").strip()


def table_md(t):
    cols = max([len(t.header)] + [len(r) for r in t.rows]) if (t.header or t.rows) else 0
    header = (t.header or [""] * cols) + [""] * (cols - len(t.header or [""] * cols))

    def row(cells):
        cells = list(cells) + [""] * (cols - len(cells))
        return "| " + " | ".join(c.replace("|", "\\|") for c in cells) + " |"

    out = []
    if t.caption:
        out.append(f"**{t.caption}**\n")
    out.append(row(header))
    out.append("|" + "---|" * cols)
    out += [row(r) for r in t.rows]
    return "\n".join(out)


# --- line classification ---------------------------------------------------------------


def heading_level(line):
    f = line.first
    if f.color != RED:
        return 0
    if f.size >= 25:
        return 1
    if f.size >= 17:
        return 2
    if f.size >= 13.5:
        return 3
    if f.size >= 11.5:
        return 4 if ("SemiBold" in f.font or "Bold" in f.font) else 5
    return 0


def is_caption(line):
    f = line.first
    return f.font == "GillSans-SemiBold" and 10.25 <= f.size <= 11 and f.color != RED


def is_sidebar_title(line):
    f = line.first
    return "SC700" in f.font and f.size >= 10.5 and f.color != RED


def is_sidebar_body(line):
    f = line.first
    return (f.font.startswith("GillSans") and f.size == 9.0) or f.font.startswith("ScalaSans")


def is_table_line(line):
    f = line.first
    if f.color in (RED, DARK_RED) or not f.font.startswith("GillSans"):
        return False
    if "SC700" in f.font:
        return False
    if f.font == "GillSans" and f.size == 10.0 and "\t" in line.text:
        return True  # a grid of terms, e.g. the six areas of effect
    return 9.2 <= f.size <= 9.8


SPELL_META = re.compile(r"^(Casting Time|Range|Components?|Duration):")


def is_skipped(line):
    f = line.first
    # The MOD/SAVE labels above a stat block's ability scores.
    if f.size <= 6.5 and f.color != DARK_RED:
        return True
    # "——Spell Slots per Spell Level——" spans the slot columns of a class table; the
    # columns' own headers (1, 2, 3…) are enough.
    return bool(re.fullmatch(r"\s*—+[^—]+—+\s*", line.text))


# --- inline markup ---------------------------------------------------------------------


def _style(span):
    bold = span.bold and not span.font.startswith("GillSans-SemiBold-SC")
    # Stat block labels (AC, HP, Skills…) are bold dark red; keep them bold.
    return (bold, span.italic)


def inline_md(spans):
    """Markdown for a run of spans, merging neighbours of the same style."""
    runs = []
    for s in spans:
        # A literal asterisk (a footnote mark) mustn't read as emphasis.
        text = s.text.replace("\t", " ").replace("\u200b", "").replace("*", "\\*")
        style = _style(s) if text.strip() else None
        if runs and (style is None or runs[-1][1] == style or runs[-1][1] is None):
            if runs[-1][1] is None:
                runs[-1][1] = style
            runs[-1][0] += text
        else:
            runs.append([text, style])
    out = []
    for text, style in runs:
        if not style or not text.strip() or style == (False, False):
            out.append(text)
            continue
        mark = {(True, False): "**", (False, True): "*", (True, True): "***"}[style]
        lead = text[: len(text) - len(text.lstrip())]
        trail = text[len(text.rstrip()) :]
        out.append(f"{lead}{mark}{text.strip()}{mark}{trail}")
    md = "".join(out)
    # Adjacent runs that both ended up bold, e.g. "**AC** **17**", read better joined.
    md = tidy(md)
    return re.sub(r"[  ]+", " ", md)


class Dehyphenator:
    """Decides whether "word-" + "next" at a line break was one word or a hyphenated one.

    It counts every word in the book: if the joined form appears somewhere, join; if the
    hyphenated form does, keep the hyphen; otherwise join, as most breaks are typesetting.
    """

    def __init__(self, all_text):
        self.words = Counter(re.findall(r"[\w’'-]+", all_text.lower()))

    def join(self, left, right):
        m = re.search(r"([\w’']+)-$", left)
        n = re.match(r"([\w’']+)", right)
        if not m or not n:
            return left + right
        a, b = m.group(1).lower(), n.group(1).lower()
        if right[:1].isupper():
            return left + right
        if self.words[f"{a}-{b}"] > self.words[a + b]:
            return left + right
        return left[:-1] + right


# --- tokenizer -------------------------------------------------------------------------


def _cells(line):
    """[(x0, x1, spans)] for each tab-separated cell of a line."""
    cells, cur = [], []
    for s in line.spans:
        if s.text == "\t":
            if cur:
                cells.append(cur)
            cur = []
        else:
            cur.append(s)
    if cur:
        cells.append(cur)
    out = []
    for c in cells:
        visible = [s for s in c if s.text.strip()] or c
        out.append((visible[0].x0, visible[-1].x1, c))
    return out


def _all_bold(line):
    return all(s.bold or not s.text.strip() for s in line.spans)


def _region(line):
    full = line.x0 < COLUMN_SPLIT - 10 and line.x1 > COLUMN_SPLIT + 10
    return (line.page, "full" if full else line.x0 >= COLUMN_SPLIT)


def build_tables(lines, caption, dehyph):
    """Builds a table that may continue into the next column or page.

    Each column's part is laid out separately (their x positions differ); parts whose
    header repeats the first one's, or that have none, are appended to it.
    """
    lines = [l for l in lines if not is_skipped(l)]
    parts, cur = [], [lines[0]]
    for line in lines[1:]:
        a, b = _region(cur[-1]), _region(line)
        if a[0] != b[0] or (a[1] != b[1] and "full" not in (a[1], b[1])):
            parts.append(cur)
            cur = []
        cur.append(line)
    parts.append(cur)
    tables = [build_table(p, "", dehyph) for p in parts]
    first = tables[0]
    first.caption = caption
    out = [first]
    for t in tables[1:]:
        same = len(t.header) == len(first.header) and [plain(h) for h in t.header] == [plain(h) for h in first.header]
        if (same or not t.header) and all(len(r) == len(first.header or first.rows[0]) for r in t.rows):
            out[-1].breaks.append(len(out[-1].rows))
            out[-1].rows += t.rows
        else:
            out.append(t)
    return out


def build_table(lines, caption, dehyph):
    """Rebuilds a table from its lines.

    Column extents come from the rows that have the most cells. Each cell joins the column
    it overlaps most. A line starting in the first column after a full row gap starts a new
    row; anything else continues the cells of the row above (wrapped text).
    """
    rows_cells = [_cells(l) for l in lines]
    widest = max(len(c) for c in rows_cells)
    model = [c for c in rows_cells if len(c) == widest]
    cols = []
    for i in range(widest):
        cols.append((min(c[i][0] for c in model), max(c[i][1] for c in model)))

    def column(x0, x1):
        best, best_score = 0, None
        for i, (c0, c1) in enumerate(cols):
            overlap = min(x1, c1) - max(x0, c0)
            score = overlap if overlap > 0 else -min(abs(x0 - c0), abs(x1 - c1), abs(x0 - c1), abs(x1 - c0))
            if best_score is None or score > best_score:
                best, best_score = i, score
        return best

    header_lines = 0
    while header_lines < len(lines) - 1 and _all_bold(lines[header_lines]):
        header_lines += 1
    if header_lines == len(lines):
        header_lines = 0

    starts = [c0 for c0, _ in cols]

    def split(line_cells):
        """Splits a cell whose text runs into the next column without a wide gap."""
        out = []
        for x0, x1, spans in line_cells:
            cur = []
            for s in spans:
                if cur and s.text.strip() and any(-9 < s.x0 - st < 2.5 for st in starts if st > x0 + 5):
                    vis = [v for v in cur if v.text.strip()]
                    if vis:
                        out.append((vis[0].x0, vis[-1].x1, cur))
                    cur = []
                cur.append(s)
            vis = [v for v in cur if v.text.strip()]
            if vis:
                out.append((vis[0].x0, vis[-1].x1, cur))
        return out

    def place(line_cells):
        row = [""] * widest
        cells = split(line_cells)
        for x0, x1, spans in cells:
            i = column(x0, x1)
            if len(cells) == 1:
                # A lone cell (wrapped text, or a group label running across columns)
                # belongs to the column it starts in.
                i = max((k for k, st in enumerate(starts) if st <= x0 + 5), default=0)
            text = inline_md(spans).strip()
            row[i] = (row[i] + " " + text).strip() if row[i] else text
        return row

    def extend(row, more):
        for i, text in enumerate(more):
            if not text:
                continue
            if row[i].endswith("\u00ad"):
                row[i] = row[i][:-1] + text
            elif row[i].endswith("-"):
                row[i] = dehyph.join(row[i], text)
            else:
                row[i] = (row[i] + " " + text).strip()

    header = [""] * widest
    for l, c in zip(lines[:header_lines], rows_cells[:header_lines]):
        extend(header, place(c))
    rows = []
    prev = lines[header_lines - 1] if header_lines else None
    for line, c in zip(lines[header_lines:], rows_cells[header_lines:]):
        placed = place(c)
        gap = line.baseline - prev.baseline if prev is not None and prev.page == line.page else 99
        new_row = not rows or (placed[0] and gap > line.size * 1.42) or (
            placed[0] and len([p for p in placed if p]) > 1 and gap > line.size * 1.2
        )
        if new_row:
            rows.append(placed)
        else:
            extend(rows[-1], placed)
        prev = line
    header = [tidy(re.sub(r"——[^—]*——\s*", "", h)) for h in header]
    rows = [[tidy(c).replace("\u00ad", "") for c in r] for r in rows]
    return Token("table", lines[0].page, caption=caption, header=header if any(header) else [], rows=rows)


class Tokenizer:
    def __init__(self, lines):
        self.lines = lines
        self.dehyph = Dehyphenator(" ".join(l.text for l in lines))

    def tokens(self):
        out = []
        i = 0
        lines = self.lines
        while i < len(lines):
            line = lines[i]
            if is_skipped(line):
                i += 1
                continue
            level = heading_level(line)
            if level:
                text = inline_md(line.spans).strip().strip("*").strip()
                # A heading broken over two lines.
                while i + 1 < len(lines) and heading_level(lines[i + 1]) == level and (
                    lines[i + 1].page == line.page and 0 < lines[i + 1].baseline - line.baseline < line.size * 1.3
                ):
                    i += 1
                    text += " " + inline_md(lines[i].spans).strip().strip("*").strip()
                    line = lines[i]
                role = ""
                if level == 3 and 14.5 < line.first.size < 15:
                    role = "statblock"
                elif level == 5:
                    role = "stat_section"
                out.append(Token("heading", line.page, level=level, md=text, role=role))
                i += 1
                continue
            if is_sidebar_title(line):
                j = i + 1
                while j < len(lines) and is_sidebar_body(lines[j]) and not is_sidebar_title(lines[j]):
                    j += 1
                body = Tokenizer.__new__(Tokenizer)
                body.lines, body.dehyph = lines[i + 1 : j], self.dehyph
                out.append(
                    Token("sidebar", line.page, caption=plain(inline_md(line.spans)), body=body._paras_only())
                )
                i = j
                continue
            if is_caption(line) or (is_table_line(line) and "\t" in line.text and not SPELL_META.match(line.text)):
                caption = ""
                if is_caption(line):
                    caption = inline_md(line.spans).strip().strip("*").strip()
                    i += 1
                j = i
                while j < len(lines) and is_table_line(lines[j]) and not SPELL_META.match(lines[j].text):
                    j += 1
                if j > i:
                    out += build_tables(lines[i:j], caption, self.dehyph)
                elif caption:
                    out.append(Token("para", line.page, md=f"**{caption}**"))
                i = j
                continue
            j = i + 1
            while j < len(lines) and self._continues(lines[j - 1], lines[j]):
                j += 1
            out.append(Token("para", line.page, md=self._join(lines[i:j])))
            i = j
        return self._reflow(out)

    @staticmethod
    def _reflow(tokens):
        """Moves sidebars and tables out of a paragraph they interrupt.

        A box at the foot of a column is read before the paragraph's continuation at the
        top of the next one; this rejoins the paragraph and puts the box after it.
        """
        out = []
        i = 0
        while i < len(tokens):
            t = tokens[i]
            if t.kind == "para" and not re.search(r"[.:!?”)*]\s*$", t.md):
                j = i + 1
                while j < len(tokens) and tokens[j].kind in ("sidebar", "table"):
                    j += 1
                bold_line = t.md.startswith("**") and t.md.endswith("**")
                if j > i + 1 and j < len(tokens) and tokens[j].kind == "para" and not bold_line and (
                    tokens[j].md[:1].islower() or not tokens[j].md.startswith(("*", "-"))
                ):
                    joined = Token("para", t.page, md=t.md + " " + tokens[j].md)
                    out.append(joined)
                    out += tokens[i + 1 : j]
                    i = j + 1
                    continue
            out.append(t)
            i += 1
        return out

    def _paras_only(self):
        """Tokens for a sidebar: paragraphs, plus tables when a row has tab-separated cells."""
        out, i, lines = [], 0, self.lines
        while i < len(lines):
            j = i + 1
            while j < len(lines) and self._continues(lines[j - 1], lines[j]):
                j += 1
            out.append(Token("para", lines[i].page, md=self._join(lines[i:j])))
            i = j
        return out

    def _starts_paragraph(self, line):
        t = line.text
        if t.startswith(" ") or t.lstrip().startswith("•") or t.lstrip().startswith("\u2022"):
            return True
        if re.match(r"^\d+: ", t):
            return True
        if SPELL_META.match(t):
            return True
        f = line.first
        if f.font.startswith("Optima-Bold") and not f.italic and f.text.strip().endswith(":"):
            return True  # a monster's spell list: "At Will:", "1/Day Each:"
        return False

    def _lead_in(self, prev, line):
        """A bold run-in heading ("**Category.** Every weapon…") after a finished sentence."""
        f = line.first
        if f.bold and re.match(r"^(Example|Step) \d+\.$", f.text.strip()):
            return True
        return (
            f.bold
            and re.search(r"[.:]”?\s*$", f.text.strip()) is not None
            and re.search(r"[.:!?”)]\s*$", prev.text.strip()) is not None
        )

    def _continues(self, prev, line):
        if heading_level(line) or is_caption(line) or is_sidebar_title(line) or is_skipped(line):
            return False
        if prev.first.bold and prev.first.size < 9.8 and prev.first.font.startswith("GillSans"):
            # A spell's Components line can wrap; anything else starts afresh.
            return (
                is_table_line(line)
                and not line.first.bold
                and not SPELL_META.match(line.text)
                and "\t" not in line.text
                and line.page == prev.page
                and 0 < line.baseline - prev.baseline < line.size * 1.45
            )
        if is_table_line(line) and "\t" in line.text and not SPELL_META.match(line.text):
            return False
        if is_table_line(line) != is_table_line(prev) and not SPELL_META.match(line.text):
            return False
        if is_sidebar_body(line) != is_sidebar_body(prev):
            return False
        if self._starts_paragraph(line) or self._lead_in(prev, line):
            return False
        if prev.first.color == DARK_RED or line.first.color == DARK_RED:
            # Stat block header lines (AC, HP, Speed…) each stand alone, apart from a
            # long one wrapping onto a line that doesn't open with a bold label.
            return prev.first.color == line.first.color == DARK_RED and not line.first.bold
        if prev.page == line.page and abs(prev.x0 - line.x0) < 60 and line.baseline > prev.baseline:
            return line.baseline - prev.baseline < max(prev.size, line.size) * 1.45
        # A column or page change: a sentence that ended on the last line probably ended
        # its paragraph too when the next line opens with a bold lead-in.
        if prev.text.rstrip().endswith((".", ":", "”")) and line.first.bold:
            return False
        return True

    def _join(self, lines):
        md = ""
        for line in lines:
            part = inline_md(line.spans)
            if not md:
                md = part.strip()
                continue
            hy = re.search(r"\w-(\**)$", md)
            if hy:
                part = part.lstrip()
                marks = re.match(r"\**", part).group(0)
                left, right = md[: len(md) - len(hy.group(1))], part[len(marks) :]
                joined = self.dehyph.join(left, right)
                if hy.group(1) and hy.group(1) == marks:
                    md = joined  # one emphasis run broken over the line
                else:
                    cut = len(left) - (len(joined) - len(right))
                    md = left[: len(left) - cut] + hy.group(1) + marks + right
            elif md.rstrip().endswith(("—", "\u00ad")):
                # An em dash, or a soft hyphen marking where a word was broken.
                md = md.rstrip().removesuffix("\u00ad") + part.lstrip()
            else:
                md = md.rstrip() + " " + part.lstrip()
        md = tidy(md).replace("\u00ad", "")
        md = re.sub(r"^(\*\*)?•(\*\*)?\s*", "- ", md)
        return md.strip()
