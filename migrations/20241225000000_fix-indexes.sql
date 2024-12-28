-- Add migration script here
DROP INDEX IF EXISTS date_tags_index;
CREATE INDEX IF NOT EXISTS date_tags_index ON pastebin (tags, date DESC);
CREATE INDEX IF NOT EXISTS user_id_date_index ON pastebin (user_id, date);
