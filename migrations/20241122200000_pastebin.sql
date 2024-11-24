-- Add migration script here
CREATE TABLE IF NOT EXISTS "pastebin" (
	"id" SERIAL PRIMARY KEY,
	"paste_id" varchar(12) NOT NULL UNIQUE,
	"user_id" varchar(256),
	"title" varchar(50),
	"tags" varchar(15) ARRAY[15],
	"format" varchar(5) NOT NULL,
	"date" timestamp with time zone NOT NULL,
	"gdriveid" varchar(384),
	"rcscore" numeric(2, 1) CHECK (rcscore >= 0.0 AND rcscore <= 1.0) NOT NULL
);

CREATE INDEX IF NOT EXISTS paste_id_index ON pastebin(paste_id);
CREATE INDEX IF NOT EXISTS tags_index ON pastebin USING GIN ("tags");
CREATE INDEX IF NOT EXISTS date_tags_index ON pastebin(tags, date);
