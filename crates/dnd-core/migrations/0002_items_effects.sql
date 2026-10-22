-- Items with weapon/armor stats, charges, modifiers and actions; feature modifiers and
-- actions; castable spells; and effects active on a character. Nested lists are stored as
-- JSON since they're only ever read and written whole.

ALTER TABLE characters ADD COLUMN round INTEGER NOT NULL DEFAULT 0;
ALTER TABLE characters ADD COLUMN riders_used_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE items ADD COLUMN kind TEXT NOT NULL DEFAULT 'gear';
ALTER TABLE items ADD COLUMN equipped INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN weapon_json TEXT;
ALTER TABLE items ADD COLUMN armor TEXT;
ALTER TABLE items ADD COLUMN ac_bonus INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN charges_max INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN charges_spent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN recharge TEXT NOT NULL DEFAULT 'none';
ALTER TABLE items ADD COLUMN modifiers_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE items ADD COLUMN actions_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE features ADD COLUMN modifiers_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE features ADD COLUMN actions_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE spells ADD COLUMN casting_time TEXT NOT NULL DEFAULT 'action';
ALTER TABLE spells ADD COLUMN free_uses_max INTEGER NOT NULL DEFAULT 0;
ALTER TABLE spells ADD COLUMN free_uses_spent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE spells ADD COLUMN roll TEXT NOT NULL DEFAULT '';
ALTER TABLE spells ADD COLUMN roll_effect TEXT NOT NULL DEFAULT 'none';
ALTER TABLE spells ADD COLUMN effect_json TEXT;

CREATE TABLE effects (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    concentration INTEGER NOT NULL,
    rounds INTEGER NOT NULL,
    modifiers_json TEXT NOT NULL,
    notes TEXT NOT NULL,
    PRIMARY KEY (character_id, position)
);
