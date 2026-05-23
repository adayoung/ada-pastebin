-- Add migration script here
ALTER TABLE "pastebin" ADD COLUMN IF NOT EXISTS "encrypted" boolean DEFAULT false NOT NULL;
