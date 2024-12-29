-- Add migration script here
CREATE TABLE IF NOT EXISTS "api_tokens" (
    "id" SERIAL PRIMARY KEY,
    "user_id" varchar(256) NOT NULL UNIQUE,
    "token" varchar(256) NOT NULL,
    "created_at" timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL
);
