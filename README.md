# CharSheet

A KDE desktop app for Dungeons & Dragons 5e (2024 rules) character sheets, written in Rust
with Qt 6 and Kirigami ([cxx-qt](https://github.com/KDAB/cxx-qt)).

You enter ability scores, classes and gear. The sheet works out modifiers, proficiency,
saves, skills, AC, initiative, spell save DC and attack, and spell slots (including
multiclassing and Pact Magic). It also tracks HP, hit dice, death saves, slots, feature
uses, conditions and Exhaustion, and rolls dice. Click any modifier to roll it. Every roll,
including those made through the MCP server, throws 3D dice across the window that land on
the rolled numbers (turn them off with **3D Dice** in the sheet's menu).

Items, features and spells can carry **modifiers** (AC, attack and damage bonuses, damage
riders such as Hunter's Mark or once-per-turn ones such as Slash Draw, speed, saves, max HP,
resistances, notes) and **actions** that spend uses, charges or quantity, roll dice, heal and
apply **effects** to the character. Equipped weapons become attacks with every modifier worked
in, including reloading and choosable ammunition. The Combat tab is the play surface: the
round counter, active effects and concentration, and every action grouped by Action, Bonus
Action and Reaction.

The whole SRD 5.2.1 is built in. **From Compendium** on the Spells, Inventory and Features
tabs searches it and adds an entry with its stats and text. Modifiers and actions are filled
in only where they're certain: from structured data, or from a hand-checked table that quotes
the rules. Everything else stays as text. Values that scale, such as proficiency-bonus uses
and cantrip dice, keep following the character through **formulas**, and items that recharge
at dawn roll for their charges on a long rest. Conditions, weapon properties, masteries and
spell names open their rules text, as does any `srd:` link in Markdown, such as
`[Prone](srd:condition:prone)`.

`examples/sable.json` is a complete level 6 Ronin Ranger built this way; import it from the
library page to see everything in use.

## Building

Fedora packages needed:

```sh
sudo dnf install cargo gcc-c++ qt6-qtbase-devel qt6-qtdeclarative-devel qt6-qtquick3d-devel \
    kf6-kirigami kf6-qqc2-desktop-style kf6-kcolorscheme-devel kf6-kconfig-devel
```

Set `KF6_INCLUDE_DIR` if the KDE Frameworks headers aren't in `/usr/include/KF6`.

```sh
cargo run -p charsheet                  # debug build
cargo test -p dnd-core                  # rules engine tests
cargo build --release
```

## Installing for your user

```sh
make install       # builds a release binary and installs to ~/.local
make uninstall     # removes it again; your characters are kept
```

This installs `~/.local/bin/charsheet` and `~/.local/bin/charsheet-mcp` plus the launcher, icon and AppStream metadata, so
CharSheet shows up in the application launcher. Make sure `~/.local/bin` is on your `PATH`.
For a system-wide install use `sudo make install PREFIX=/usr/local`. `DESTDIR` works for
staged (packaging) installs.

Other targets: `make run`, `make test`, `make clean`.

## Usage

```sh
charsheet                               # open the character library
charsheet --open "Brakka" --tab 1       # open a character by name or id on a tab (0-6)
```

The ⋮ menu's Theme entry switches between *Stone*, styled after the 2024 character sheet,
*Parchment*, styled after the Player's Handbook, and *System*, which uses your desktop's
colors and fonts.

Characters and their recent dice rolls are stored in a SQLite database at
`~/.local/share/charsheet/charsheet.db`. Use Export/Import on the library page to share a
character as a JSON file.

## MCP server

`charsheet-mcp` is a [Model Context Protocol](https://modelcontextprotocol.io) server over
stdio, so an AI assistant can read, edit and play your characters. To add it to Claude Code:

```sh
claude mcp add charsheet -- ~/.local/bin/charsheet-mcp
```

It works on the same library as the app (`--db <path>` picks another one). Changes are saved
immediately, and the server tells any running app, so a character open there updates at
once.

- **Library:** `list_characters`, `create_character`, `duplicate_character`,
  `delete_character`, `import_character`, `export_character`
- **Reading:** `get_character` (the whole sheet with derived values, or one part by JSON
  Pointer such as `/inventory` or `/derived/weapon_attacks`), `get_catalog` (valid choices)
- **Rules:** `search_srd`, `get_srd`, and `add_from_srd` to put a spell, item or feature on
  a character
- **Editing:** `set_field`, `append_item`, `remove_item`, `move_item` by JSON Pointer
  (`/hp/max`, `/skills/stealth`, `/inventory/2/modifiers`), and `apply_edits` for several
  at once, all or nothing
- **Play:** `damage`, `heal`, `set_temp_hp`, `spend_hit_die`, `short_rest`, `long_rest`,
  `roll`, `get_rolls`, `clear_rolls`, `use_item_action`, `use_feature_action`,
  `cast_spell`, `weapon_attack`, `weapon_damage`, `reload`, `apply_effect`, `end_effect`,
  `new_turn`, `end_combat`

## Layout

- `crates/dnd-core`: the model, rules (SRD 5.2 tables), dice, JSON-pointer edits, play
  actions, storage and the embedded SRD (`src/srd/`). It has no Qt dependency and is
  unit-tested.
- `crates/charsheet`: the Qt app. `src/bridge/` exposes `CharacterLibrary`,
  `CharacterSheet` and `Srd` to QML, and `qml/` holds the Kirigami UI. `cpp/theme.h` applies the
  bundled color schemes in `theme/`, and `qml/SheetFrame.qml` and its neighbours draw the
  sheet decorations. `tools/make_textures.py` generates the background textures.
- `crates/charsheet-mcp`: the MCP server, built on `dnd-core` with
  [rmcp](https://crates.io/crates/rmcp).
- `data/srd`: the whole SRD 5.2.1 as structured, searchable JSON (spells, monsters, magic
  items, classes, equipment, rules text and more), generated from the SRD PDF by
  `tools/srd` with `make srd` and compiled into `dnd-core`. See `data/srd/README.md` for
  the format.

This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by
Wizards of the Coast LLC, available at <https://www.dndbeyond.com/srd>. The SRD 5.2.1 is
licensed under the Creative Commons Attribution 4.0 International License, available at
<https://creativecommons.org/licenses/by/4.0/legalcode>. It has been reformatted as
structured data, and the app's rules dialogs credit it.

The bundled fonts in `crates/charsheet/fonts/` are Solbera's 5e font imitations (with fixes
by Ryrok, Ners and LUCASTUCIOUS), licensed under CC-BY-SA-4.0; see the README there.
