-- Create games table
CREATE TABLE IF NOT EXISTS games
(
    id
    BIGSERIAL
    PRIMARY
    KEY,
    game_hash
    TEXT
    NOT
    NULL,
    created
    TIMESTAMP
    WITH
    TIME
    ZONE
    NOT
    NULL
    DEFAULT
    CURRENT_TIMESTAMP,
    did
    TEXT
    NOT
    NULL,
    at_uri
    TEXT
    NOT
    NULL,
    record
    JSONB
    NOT
    NULL
);

-- Create unique indexes
CREATE UNIQUE INDEX games_game_hash_idx ON games (game_hash);
CREATE UNIQUE INDEX games_at_uri_idx ON games (at_uri);

-- Create non-unique index
CREATE INDEX games_did_idx ON games (did);