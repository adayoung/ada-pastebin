-- Add migration script here
ALTER TABLE "pastebin" ADD COLUMN IF NOT EXISTS "session_id" varchar(12);
