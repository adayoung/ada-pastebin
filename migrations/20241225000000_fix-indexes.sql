-- Add migration script here
DROP INDEX IF EXISTS date_tags_index;
CREATE INDEX date_tags_index ON pastebin (tags, date DESC);
CREATE INDEX user_id_date_index ON pastebin (user_id, date);
