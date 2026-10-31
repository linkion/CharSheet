-- Formulas keep values following the character; regain is the dice of a dawn recharge.
ALTER TABLE spells ADD COLUMN roll_formula TEXT NOT NULL DEFAULT '';
ALTER TABLE features ADD COLUMN uses_formula TEXT NOT NULL DEFAULT '';
ALTER TABLE items ADD COLUMN regain TEXT NOT NULL DEFAULT '';
