# SRD 5.2.1 data

The System Reference Document 5.2.1 (the D&D 2024 rules released under CC-BY-4.0) as
structured JSON. Every part of the book is here: typed records for the things a character
sheet uses (spells, classes, species, feats, equipment, magic items, monsters) and the rest
of the rules text as a tree of sections. `index.json` lists all of it for search.

`dnd-core` compiles these files in (`crates/dnd-core/src/srd/`), so rebuild after regenerating.
`mechanics.json` there holds hand-checked mechanics for records that are only prose. Each
entry quotes the text it rests on, and a test fails if that text changes.

The files are generated; don't edit them by hand. Regenerate them with `make srd` (see
[Regenerating](#regenerating)).

## Files

| File | Records | What |
|---|---|---|
| `index.json` | 1,777 | One row per record in every other file, for search |
| `spells.json` | 339 | Spells |
| `monsters.json` | 330 | Stat blocks from Monsters A–Z (`group: "monster"`) and Animals (`group: "animal"`) |
| `magic_items.json` | 258 | Magic Items A–Z |
| `rules.json` | 392 | Every other section of the book: Playing the Game, Character Creation, Gameplay Toolbox, and the introductions to each chapter |
| `glossary.json` | 155 | Rules Glossary entries |
| `conditions.json` | 15 | The conditions, with their effects split out |
| `classes.json` | 12 | Classes, with their feature tables, features, option lists and spell lists |
| `subclasses.json` | 12 | Subclasses, with features and always-prepared spells |
| `species.json` | 9 | Species and their traits |
| `backgrounds.json` | 4 | Backgrounds |
| `feats.json` | 17 | Feats |
| `weapons.json` | 38 | The Weapons table |
| `weapon_properties.json` | 10 | Weapon properties (Finesse, Light, …) |
| `mastery_properties.json` | 8 | Weapon mastery properties (Cleave, Graze, …) |
| `armor.json` | 13 | The Armor table, including the Shield |
| `tools.json` | 25 | Artisan's tools and other tools |
| `gear.json` | 81 | Adventuring gear, with variants (ammunition, focuses, holy symbols) |
| `mounts_vehicles.json` | 25 | Mounts, tack and drawn vehicles, and ships |
| `services.json` | 34 | Lifestyles, food and lodging, hirelings, spellcasting services |
| `manifest.json` | | Source, version, attribution, generation time and per-file counts |
| `schema/*.schema.json` | | A JSON Schema (draft 2020-12) for each file |

Each data file is an array of records sorted by name, except `rules.json`, which is in book
order.

## Records

Every record has these fields:

| Field | Meaning |
|---|---|
| `id` | Stable id, `category:slug`, e.g. `spell:acid-arrow`, `monster:adult-red-dragon`, `class:wizard`. Rules ids are paths: `rule:playing-the-game/d20-tests/ability-checks` |
| `name` | The name as printed |
| `category` | `spell`, `monster`, `magic_item`, `rule`, `glossary`, `condition`, `class`, `subclass`, `species`, `background`, `feat`, `weapon`, `weapon_property`, `mastery_property`, `armor`, `tool`, `gear`, `mount`, `vehicle` or `service` |
| `page` | The page it starts on in the PDF |
| `summary` | A sentence for search results |
| `markdown` | The full text as Markdown: bold, italics, lists, tables and sidebars (as block quotes) kept |

Records refer to each other by id: a spell's `classes`, a class's `spell_list` and
`subclasses`, a subclass's `class`, a weapon's `properties` and `mastery`, a background's
`feat`, a glossary entry's `see_also`. `check.py` verifies that every reference resolves.

Enum values are snake_case, as in the app: abilities (`strength` … `charisma`), skills
(`sleight_of_hand`), damage types, conditions, schools, sizes and creature types.

### Field notes

- **Spells.** `casting_time.activation` is `action`, `bonus_action`, `reaction` (with the
  `trigger`) or `time` (with `amount` and `unit`). `range.kind` is `self`, `touch`, `feet`,
  `miles`, `sight`, `unlimited` or `special`. `duration.kind` is `instantaneous`, `timed`,
  `until_dispelled` or `special`. `saves`, `attack`, `damage` and `damage_types` are read
  from the description and are a best effort; the text in `markdown` is authoritative.
- **Monsters.** `abilities` holds `str` … `cha` with `score`, `mod` and `save`. `cr.value`
  is a number (`0.125` for CR 1/8) and `cr.text` is as printed. Each trait and action has
  `name`, optional `usage` ("3/Day", "Recharge 5–6"), and where present `attack` (kind,
  bonus, reach, range), `save` (ability, DC) and `damage` (average, dice, type).
  Conditional speeds ("40 ft. (bear form only)") and qualified defences stay as text in
  `notes`.
- **Magic items.** `rarity` is `varies` when it depends on the variant; then
  `rarity_by_variant` lists each (`+1` → `rare`, …). `aliases` holds names such as
  "+1 Armor" for "Armor, +1, +2, or +3".
- **Armor.** `ac.dex_cap` is `null` for no cap and `0` when Dexterity isn't added. The
  Shield has `ac.bonus` instead of `ac.base`.
- **Classes.** `features_table.rows` has one row per level with `features`, `spell_slots`
  (levels 1–9) and the class's own columns in snake_case (`rages`, `sneak_attack`,
  `pact_slots`, …).
- **Costs and weights.** `cost` is `{amount, unit}` with unit `cp`, `sp`, `ep`, `gp` or `pp`;
  `weight_lb` is a number of pounds. Either is `null` where the book prints "—" or
  "Varies".
- **Rules.** Each heading is one record holding only its own text, with `parent`,
  `children` and `path` to rebuild the tree.

## Searching

`index.json` is small enough to load whole. Each row is
`{id, name, category, aliases, keywords, page, file, summary}`, where `keywords` holds
facets such as a spell's level, school and classes, a monster's type and CR, or an item's
rarity. Look up the full record by `id` in `file`. The rows map directly onto a SQLite FTS5
table (the app's bundled SQLite includes FTS5) if a full-text index is wanted later.

## Regenerating

The PDF isn't in the repository. Download `SRD_CC_v5.2.1.pdf` from
<https://www.dndbeyond.com/srd>, put it in the repository root and run:

```sh
make srd                                   # build, then check
python3 -I tools/srd/build.py SRD_CC_v5.2.1.pdf data/srd
python3 -I tools/srd/check.py data/srd SRD_CC_v5.2.1.pdf
```

This needs Python 3 and `mutool` (MuPDF); nothing else. The build is deterministic apart
from `generated_at` in `manifest.json`.

How it works (`tools/srd/`):

- `layout.py` reads every character's font, size, colour and position from mutool, and
  puts each page's lines in reading order across its two columns and full-width tables.
- `markdown.py` turns the lines into headings, paragraphs, tables and sidebars.
- `sections.py` builds the heading tree.
- `parsers/` turns each chapter into typed records.
- `schema.py` defines the schemas.
- `check.py` validates the result. It checks the schemas and references, and the expected
  counts. It cross-checks the class spell lists against each spell's classes. It also
  matches every monster against the book's Index of Stat Blocks, page included.

Misprints in the PDF are fixed in `tools/srd/overrides.json`, which is deep-merged into
records by id. One known quirk is left as printed: the class spell lists leave out
*Phantasmal Force*, though the spell itself names the Bard, Sorcerer and Wizard.

## License

This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by
Wizards of the Coast LLC, available at <https://www.dndbeyond.com/srd>. The SRD 5.2.1 is
licensed under the Creative Commons Attribution 4.0 International License, available at
<https://creativecommons.org/licenses/by/4.0/legalcode>.
