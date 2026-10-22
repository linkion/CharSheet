-- Counts every write to a character (sheet or roll log), so another process with the
-- character open can tell it changed.
ALTER TABLE characters ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
