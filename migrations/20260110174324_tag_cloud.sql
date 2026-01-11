-- Add migration script here
CREATE MATERIALIZED VIEW IF NOT EXISTS popular_tags AS
SELECT tag, COUNT(*) AS frequency
FROM (
   SELECT unnest(tags) AS tag
   FROM pastebin
) AS flattened_tags
GROUP BY tag
ORDER BY frequency DESC, tag;

CREATE UNIQUE INDEX ON popular_tags(tag);

-- Might have to run these manually!
-- CREATE EXTENSION IF NOT EXISTS pg_cron;
-- SELECT cron.schedule('0 0 * * *', 'REFRESH MATERIALIZED VIEW popular_tags;');
