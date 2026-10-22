# CharSheet

A KDE desktop app for Dungeons & Dragons 5e (2024 rules) character sheets, written in Rust
with Qt 6 and Kirigami ([cxx-qt](https://github.com/KDAB/cxx-qt)).

You enter ability scores, classes and gear. The sheet works out modifiers, proficiency,
saves, skills, AC, initiative, spell save DC and attack, and spell slots (including
multiclassing and Pact Magic). It also tracks HP, hit dice, death saves, slots, feature
uses, conditions and Exhaustion, and rolls dice. Click any modifier to roll it.

## Building

Fedora packages needed:

```sh
sudo dnf install cargo gcc-c++ qt6-qtbase-devel qt6-qtdeclarative-devel \
    kf6-kirigami kf6-qqc2-desktop-style
```

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

This installs `~/.local/bin/charsheet` plus the launcher, icon and AppStream metadata, so
CharSheet shows up in the application launcher. Make sure `~/.local/bin` is on your `PATH`.
For a system-wide install use `sudo make install PREFIX=/usr/local`. `DESTDIR` works for
staged (packaging) installs.

Other targets: `make run`, `make test`, `make clean`.

## Usage

```sh
charsheet                               # open the character library
charsheet --open "Brakka" --tab 1       # open a character by name or id on a tab (0-6)
```

Characters and their recent dice rolls are stored in a SQLite database at
`~/.local/share/charsheet/charsheet.db`. Use Export/Import on the library page to share a
character as a JSON file.

## Layout

- `crates/dnd-core`: the model, rules (SRD 5.2 tables), dice, JSON-pointer edits and
  storage. It has no Qt dependency and is unit-tested.
- `crates/charsheet`: the Qt app. `src/bridge/` exposes `CharacterLibrary` and
  `CharacterSheet` to QML, and `qml/` holds the Kirigami UI.

Rules data comes from the System Reference Document 5.2 by Wizards of the Coast,
licensed under CC-BY-4.0.
