CREATE TABLE IF NOT EXISTS server_keys (
    server_id UUID REFERENCES servers(id),
    user_id UUID REFERENCES users(id),
    encrypted_key TEXT NOT NULL,
    PRIMARY KEY(server_id, user_id)
);
