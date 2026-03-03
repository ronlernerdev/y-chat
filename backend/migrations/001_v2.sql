-- Messy V2 Schema
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    uname VARCHAR(255) UNIQUE NOT NULL,
    pass VARCHAR(255) NOT NULL,
    pubkey TEXT NOT NULL,
    avatar TEXT
);

CREATE TABLE IF NOT EXISTS servers (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    owner UUID REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS channels (
    id UUID PRIMARY KEY,
    server_id UUID REFERENCES servers(id),
    name VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS members (
    user_id UUID REFERENCES users(id),
    server_id UUID REFERENCES servers(id),
    PRIMARY KEY(user_id, server_id)
);

CREATE TABLE IF NOT EXISTS v2_messages (
    id UUID PRIMARY KEY,
    channel_id UUID REFERENCES channels(id),
    author_id UUID REFERENCES users(id),
    encrypted_content TEXT NOT NULL,
    nonce TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
