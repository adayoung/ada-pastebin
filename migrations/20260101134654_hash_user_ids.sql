-- Add migration script here
CREATE EXTENSION IF NOT EXISTS pgcrypto;
UPDATE api_tokens
SET user_id = 'sha256-' || encode(digest(COALESCE(user_id, ''), 'sha256'), 'hex')
WHERE user_id NOT LIKE 'sha256-%';

UPDATE pastebin
SET user_id = 'sha256-' || encode(digest(COALESCE(user_id, ''), 'sha256'), 'hex')
WHERE user_id IS NOT NULL AND LENGTH(user_id) < 64 AND user_id NOT LIKE 'sha256-%';

-- Update old user_id hashes made from the last pastebin to the new format --
-- https://github.com/adayoung/gae-pastebin/commit/d440ac9d5940b9b0ff659e160e9b2a8389bc2c66
UPDATE pastebin
SET user_id = 'sha256-' || user_id
WHERE LENGTH(user_id) >= 64 AND user_id NOT LIKE 'sha256-%';
