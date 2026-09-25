CREATE TABLE IF NOT EXISTS tlab_refresh_token (
    user_account_id BIGINT PRIMARY KEY REFERENCES tlab_user_account (id),
    token_hash BYTEA NOT NULL CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL
);

COMMENT ON TABLE tlab_refresh_token IS '사용자 계정별 현재 유효한 refresh token';
COMMENT ON COLUMN tlab_refresh_token.user_account_id IS 'refresh token을 소유한 사용자 계정 ID';
COMMENT ON COLUMN tlab_refresh_token.token_hash IS 'refresh token의 SHA-256 해시';
COMMENT ON COLUMN tlab_refresh_token.created_at IS 'refresh token 저장 시각';
COMMENT ON COLUMN tlab_refresh_token.expires_at IS 'refresh token 만료 시각';
