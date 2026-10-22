CREATE TABLE characters (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    player TEXT NOT NULL,
    species TEXT NOT NULL,
    background TEXT NOT NULL,
    alignment TEXT NOT NULL,
    jack_of_all_trades INTEGER NOT NULL,
    hp_max INTEGER NOT NULL,
    hp_current INTEGER NOT NULL,
    hp_temp INTEGER NOT NULL,
    death_successes INTEGER NOT NULL,
    death_failures INTEGER NOT NULL,
    heroic_inspiration INTEGER NOT NULL,
    exhaustion INTEGER NOT NULL,
    speed INTEGER NOT NULL,
    armor TEXT,
    shield INTEGER NOT NULL,
    ac_bonus INTEGER NOT NULL,
    initiative_bonus INTEGER NOT NULL,
    pact_slots_used INTEGER NOT NULL,
    cp INTEGER NOT NULL,
    sp INTEGER NOT NULL,
    ep INTEGER NOT NULL,
    gp INTEGER NOT NULL,
    pp INTEGER NOT NULL,
    languages TEXT NOT NULL,
    other_proficiencies TEXT NOT NULL,
    appearance TEXT NOT NULL,
    backstory TEXT NOT NULL,
    notes TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE character_classes (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    class TEXT NOT NULL,
    subclass TEXT NOT NULL,
    level INTEGER NOT NULL,
    hit_dice_spent INTEGER NOT NULL,
    third_caster INTEGER NOT NULL,
    PRIMARY KEY (character_id, position)
);

CREATE TABLE character_abilities (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    ability TEXT NOT NULL,
    base_score INTEGER NOT NULL,
    bonus INTEGER NOT NULL,
    extra_save INTEGER NOT NULL,
    PRIMARY KEY (character_id, ability)
);

CREATE TABLE character_skills (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    skill TEXT NOT NULL,
    proficiency TEXT NOT NULL,
    PRIMARY KEY (character_id, skill)
);

CREATE TABLE character_conditions (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    condition TEXT NOT NULL,
    PRIMARY KEY (character_id, condition)
);

CREATE TABLE character_spell_slots (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    level INTEGER NOT NULL CHECK (level BETWEEN 1 AND 9),
    used INTEGER NOT NULL,
    PRIMARY KEY (character_id, level)
);

CREATE TABLE attacks (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    ability TEXT NOT NULL,
    proficient INTEGER NOT NULL,
    damage TEXT NOT NULL,
    damage_type TEXT NOT NULL,
    bonus INTEGER NOT NULL,
    add_ability_to_damage INTEGER NOT NULL,
    mastery TEXT NOT NULL,
    notes TEXT NOT NULL,
    PRIMARY KEY (character_id, position)
);

CREATE TABLE spells (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    level INTEGER NOT NULL,
    prepared INTEGER NOT NULL,
    ritual INTEGER NOT NULL,
    concentration INTEGER NOT NULL,
    notes TEXT NOT NULL,
    PRIMARY KEY (character_id, position)
);

CREATE TABLE items (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    weight REAL NOT NULL,
    requires_attunement INTEGER NOT NULL,
    attuned INTEGER NOT NULL,
    notes TEXT NOT NULL,
    PRIMARY KEY (character_id, position)
);

CREATE TABLE features (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    description TEXT NOT NULL,
    uses_max INTEGER NOT NULL,
    uses_spent INTEGER NOT NULL,
    recharge TEXT NOT NULL,
    PRIMARY KEY (character_id, position)
);

CREATE TABLE rolls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    rolled_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    label TEXT NOT NULL,
    expression TEXT NOT NULL,
    mode TEXT NOT NULL,
    total INTEGER NOT NULL,
    natural INTEGER,
    breakdown TEXT NOT NULL,
    -- Per-term dice detail, only ever displayed.
    terms_json TEXT NOT NULL
);

CREATE INDEX rolls_by_character ON rolls (character_id, id DESC);
