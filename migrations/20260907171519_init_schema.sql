
-- 1. businesses — the tenant
CREATE TABLE businesses (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. api_keys — many per business
CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  UUID NOT NULL REFERENCES businesses(id),
    key_prefix   VARCHAR(32) NOT NULL UNIQUE,
    key_hash     VARCHAR(255) NOT NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_api_keys_business_id ON api_keys(business_id);

-- 3. customers — scoped to a business
CREATE TABLE customers (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  UUID NOT NULL REFERENCES businesses(id),
    name         VARCHAR(255) NOT NULL,
    email        VARCHAR(255) NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (business_id, email)
);

CREATE INDEX idx_customers_business_created ON customers(business_id, created_at DESC);

-- 4. invoices 
CREATE TABLE invoices (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id         UUID NOT NULL REFERENCES businesses(id),
    customer_id         UUID NOT NULL REFERENCES customers(id),
    total_amount_cents  BIGINT NOT NULL DEFAULT 0 CHECK (total_amount_cents >= 0),
    currency            VARCHAR(3) NOT NULL DEFAULT 'USD',
    state               VARCHAR(20) NOT NULL DEFAULT 'draft'
                             CHECK (state IN ('draft', 'open', 'paid', 'void', 'uncollectible')),
    due_date            DATE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invoices_business_state_created ON invoices(business_id, state, created_at DESC);

-- 5. invoice_line_items
CREATE TABLE invoice_line_items (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id         UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    position           INTEGER NOT NULL,
    description        VARCHAR(500) NOT NULL,
    quantity           INTEGER NOT NULL CHECK (quantity > 0),
    unit_amount_cents  BIGINT NOT NULL CHECK (unit_amount_cents >= 0),
    amount_cents       BIGINT GENERATED ALWAYS AS (quantity * unit_amount_cents) STORED,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (invoice_id, position)
);

CREATE INDEX idx_line_items_invoice_id ON invoice_line_items(invoice_id);

-- 6. payment_attempts 
CREATE TABLE payment_attempts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id       UUID NOT NULL REFERENCES invoices(id),
    business_id      UUID NOT NULL REFERENCES businesses(id),
    status           VARCHAR(20) NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'succeeded', 'failed')),
    amount_cents     BIGINT NOT NULL CHECK (amount_cents >= 0),
    card_token       VARCHAR(100) NOT NULL,
    psp_ref          VARCHAR(100),
    failure_code     VARCHAR(100),
    idempotency_key  VARCHAR(255) NOT NULL,
    request_hash     BYTEA NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (business_id, idempotency_key)
);

CREATE UNIQUE INDEX idx_payment_attempts_one_success_per_invoice
    ON payment_attempts(invoice_id) WHERE status = 'succeeded';

CREATE UNIQUE INDEX idx_payment_attempts_one_pending_per_invoice
    ON payment_attempts(invoice_id) WHERE status = 'pending';

CREATE INDEX idx_payment_attempts_invoice_created ON payment_attempts(invoice_id, created_at DESC);
CREATE INDEX idx_payment_attempts_pending_updated ON payment_attempts(updated_at) WHERE status = 'pending';

-- 7. webhook_endpoints
CREATE TABLE webhook_endpoints (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id  UUID NOT NULL REFERENCES businesses(id),
    url          TEXT NOT NULL,
    secret       BYTEA NOT NULL,
    active       BOOLEAN NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhook_endpoints_business_id ON webhook_endpoints(business_id);

-- 8. webhook_deliveries
CREATE TABLE webhook_deliveries (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    business_id           UUID NOT NULL REFERENCES businesses(id),
    endpoint_id           UUID NOT NULL REFERENCES webhook_endpoints(id),
    event_id              UUID NOT NULL DEFAULT gen_random_uuid(),
    event_type            VARCHAR(100) NOT NULL,
    payload               JSONB NOT NULL,
    status                VARCHAR(20) NOT NULL DEFAULT 'pending'
                              CHECK (status IN ('pending', 'succeeded', 'exhausted')),
    attempt_count         INTEGER NOT NULL DEFAULT 0,
    next_attempt_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error            TEXT,
    last_response_status  INTEGER,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_webhook_deliveries_pending_due ON webhook_deliveries(next_attempt_at) WHERE status = 'pending';
CREATE INDEX idx_webhook_deliveries_business_created ON webhook_deliveries(business_id, created_at DESC);
