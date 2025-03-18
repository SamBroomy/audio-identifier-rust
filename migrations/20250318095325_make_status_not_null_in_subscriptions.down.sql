-- Add down migration script here
BEGIN;
-- Make `status` column nullable again
ALTER TABLE subscriptions
ALTER COLUMN status DROP NOT NULL;
COMMIT;