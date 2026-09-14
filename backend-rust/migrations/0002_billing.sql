CREATE TYPE subscription_status AS ENUM (
    'TRIALING', 'PENDING_PAYMENT', 'ACTIVE', 'PAST_DUE',
    'SUSPENDED', 'CANCEL_AT_PERIOD_END', 'CANCELLED', 'EXPIRED'
);
CREATE TYPE invoice_status AS ENUM (
    'DRAFT', 'PENDING', 'PAID', 'EXPIRED', 'FAILED', 'CANCELLED',
    'REFUND_PENDING', 'REFUNDED', 'CHARGEBACK'
);

CREATE TABLE pricing_plans (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code varchar(32) UNIQUE NOT NULL,
    name varchar(80) NOT NULL,
    monthly_price_idr bigint NOT NULL CHECK (monthly_price_idr >= 0),
    analyses_per_month int NOT NULL CHECK (analyses_per_month >= 0),
    max_mt5_accounts int NOT NULL CHECK (max_mt5_accounts >= 0),
    live_eligible boolean NOT NULL DEFAULT false,
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    entitlements jsonb NOT NULL DEFAULT '{}',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE subscriptions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id),
    plan_id uuid NOT NULL REFERENCES pricing_plans(id),
    provider varchar(32),
    provider_customer_ref varchar(255),
    provider_subscription_ref varchar(255),
    status subscription_status NOT NULL,
    cancel_at_period_end boolean NOT NULL DEFAULT false,
    trial_end timestamptz,
    current_period_start timestamptz NOT NULL,
    current_period_end timestamptz NOT NULL,
    grace_period_end timestamptz,
    cancelled_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (current_period_end > current_period_start)
);
CREATE UNIQUE INDEX one_current_subscription_per_user
    ON subscriptions(user_id)
    WHERE status IN ('TRIALING', 'PENDING_PAYMENT', 'ACTIVE', 'PAST_DUE', 'CANCEL_AT_PERIOD_END');
CREATE UNIQUE INDEX subscriptions_provider_ref
    ON subscriptions(provider, provider_subscription_ref)
    WHERE provider_subscription_ref IS NOT NULL;

CREATE TABLE invoices (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id uuid NOT NULL REFERENCES subscriptions(id),
    invoice_number varchar(64) UNIQUE NOT NULL,
    provider_invoice_ref varchar(255),
    currency char(3) NOT NULL DEFAULT 'IDR',
    subtotal_minor bigint NOT NULL CHECK (subtotal_minor >= 0),
    tax_minor bigint NOT NULL DEFAULT 0 CHECK (tax_minor >= 0),
    total_minor bigint NOT NULL CHECK (total_minor >= 0),
    status invoice_status NOT NULL,
    due_at timestamptz,
    paid_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (total_minor = subtotal_minor + tax_minor)
);
CREATE UNIQUE INDEX invoices_provider_ref
    ON invoices(provider_invoice_ref)
    WHERE provider_invoice_ref IS NOT NULL;
CREATE INDEX subscriptions_status_period ON subscriptions(status, current_period_end);
CREATE INDEX invoices_status_created ON invoices(status, created_at DESC);

INSERT INTO pricing_plans(code, name, monthly_price_idr, analyses_per_month, max_mt5_accounts, live_eligible, entitlements)
VALUES
    ('FREE_TRIAL', 'Free Trial', 0, 20, 1, false, '{"history_days":7,"support":"self_service"}'),
    ('DEMO', 'Demo', 99000, 150, 1, false, '{"history_days":90,"support":"standard"}'),
    ('PRO', 'Pro', 249000, 1000, 3, true, '{"history_days":365,"support":"priority"}'),
    ('BUSINESS', 'Business', 799000, 5000, 10, true, '{"history_days":730,"support":"contract"}')
ON CONFLICT (code) DO NOTHING;

