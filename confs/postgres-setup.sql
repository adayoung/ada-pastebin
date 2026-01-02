CREATE SCHEMA pastebin;
CREATE ROLE ada WITH LOGIN PASSWORD '<stick a super sekret password here!>';

GRANT USAGE ON SCHEMA pastebin TO ada;
GRANT CREATE ON SCHEMA pastebin TO ada;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA pastebin TO ada;
ALTER DEFAULT PRIVILEGES IN SCHEMA pastebin GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ada;

-- GRANT USAGE ON SCHEMA extensions TO ada;  # adjust as needed
-- ALTER ROLE ada SET search_path TO pastebin, extensions;  # adjust as needed
