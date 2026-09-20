CREATE TYPE admin_role AS ENUM ('OWNER','SECURITY_OPERATOR','OPS_ADMIN','SUPPORT_AGENT','AUDITOR');

CREATE TABLE admin_users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username varchar(64) UNIQUE NOT NULL,
    password_hash text NOT NULL,
    role admin_role NOT NULL,
    totp_secret_encrypted text NOT NULL,
    totp_secret_nonce text NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    created_by uuid REFERENCES admin_users(id),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE admin_sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_user_id uuid NOT NULL REFERENCES admin_users(id),
    token_hash varchar(64) NOT NULL UNIQUE,
    csrf_token varchar(64) NOT NULL,
    ip inet,
    user_agent varchar(255),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz
);
CREATE INDEX admin_sessions_expiry ON admin_sessions(expires_at);

CREATE TABLE admin_audit_logs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_user_id uuid REFERENCES admin_users(id),
    action varchar(64) NOT NULL,
    target_type varchar(32) NOT NULL,
    target_id uuid,
    reason text,
    metadata jsonb NOT NULL DEFAULT '{}',
    ip inet,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX admin_audit_created ON admin_audit_logs(created_at DESC);

CREATE TABLE account_credits (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id),
    amount_minor bigint NOT NULL CHECK (amount_minor > 0),
    currency char(3) NOT NULL DEFAULT 'IDR',
    reason text NOT NULL,
    granted_by uuid NOT NULL REFERENCES admin_users(id),
    created_at timestamptz NOT NULL DEFAULT now()
);
