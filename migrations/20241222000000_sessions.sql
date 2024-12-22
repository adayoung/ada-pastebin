-- Add migration script here
ALTER TABLE "pastebin" ADD COLUMN "session_id" varchar(12);
