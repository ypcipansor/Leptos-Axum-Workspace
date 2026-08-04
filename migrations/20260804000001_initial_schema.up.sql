-- Initial schema.
--
-- Replaces the previous approach of issuing CREATE TABLE IF NOT EXISTS from
-- application startup code, which left the schema unversioned and impossible to
-- evolve: there was no record of what had been applied and no way to change a
-- column without hand-editing production.

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------
CREATE TABLE users (
    -- A surrogate key. The previous schema used `username` as the primary key
    -- and had sessions reference it, which meant a rename would either be
    -- refused or would cascade through every dependent row.
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT        NOT NULL,
    -- Argon2id PHC string. It embeds the algorithm, version, parameters and
    -- salt, so parameters can be raised later and old hashes still verify.
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Mirrors Username::parse in crates/core. The application always validates
    -- first; this is the backstop for anything reaching the table by another
    -- route (a migration, a manual fix, a future service).
    CONSTRAINT users_username_len_chk
        CHECK (char_length(username) BETWEEN 3 AND 32),
    CONSTRAINT users_username_charset_chk
        CHECK (username ~ '^[A-Za-z0-9][A-Za-z0-9._-]*$')
);

-- Case-insensitive uniqueness. A functional unique index avoids depending on
-- the citext extension, which needs privileges a managed database may not grant.
CREATE UNIQUE INDEX users_username_lower_key ON users (lower(username));

-- ---------------------------------------------------------------------------
-- sessions
-- ---------------------------------------------------------------------------
CREATE TABLE sessions (
    -- Public identifier, safe to send to the browser and to address in revoke
    -- requests. Distinct from the token, which never leaves the cookie.
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- SHA-256 of the session token, never the token itself. A dump of this
    -- table yields nothing an attacker can present as a credential. The
    -- previous schema stored the bearer token verbatim as the primary key.
    token_hash   BYTEA       NOT NULL,

    user_agent   TEXT,
    -- INET rather than text: Postgres validates the value and can index it.
    -- NULL means the address was unknown or came from an untrusted hop, which
    -- is recorded honestly instead of being backfilled with a placeholder.
    ip_address   INET,

    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Sessions previously had no expiry at all and lived until explicitly
    -- revoked. NOT NULL makes an immortal session unrepresentable.
    expires_at   TIMESTAMPTZ NOT NULL,

    CONSTRAINT sessions_token_hash_len_chk CHECK (octet_length(token_hash) = 32),
    CONSTRAINT sessions_expiry_after_creation_chk CHECK (expires_at > created_at)
);

-- Every authenticated request looks a session up by token hash, so this index
-- carries the hot path. Unique because a hash collision would be a security
-- incident, not a merge.
CREATE UNIQUE INDEX sessions_token_hash_key ON sessions (token_hash);

-- Serves the "your active sessions" list, already in display order.
CREATE INDEX sessions_user_id_created_at_idx ON sessions (user_id, created_at DESC);

-- Serves the periodic cleanup sweep.
CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

-- ---------------------------------------------------------------------------
-- updated_at maintenance
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Keeping this in the database means updated_at stays correct no matter which
-- code path writes the row.
CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
