-- Add migration script here
DROP INDEX IF EXISTS paste_id_index;
DROP INDEX IF EXISTS tags_index;
CREATE INDEX date_index ON "pastebin" ("date" desc);
CREATE INDEX tags_index ON "pastebin" USING GIN ((tags));
