-- The type of damage a spell's roll deals, shown in its roll's breakdown.

ALTER TABLE spells ADD COLUMN damage_type TEXT NOT NULL DEFAULT '';
