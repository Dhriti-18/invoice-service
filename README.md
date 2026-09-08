# Invoice & Payment Service

A minimal invoice + payment service: a business creates invoices for customers, customers pay invoices, and the business gets notified of state changes via signed webhooks. See [`DESIGN.md`](DESIGN.md) for the data model, state machine, failure-mode reasoning, and what was deliberately cut.

## Run it

Requires Docker and Docker Compose.

```bash
docker compose up --build
```

This brings up three containers with no further steps: Postgres (migrations run automatically on boot), the mock PSP (port `9090`), and the API (port `8080`).

Once it's up, create a business and get an API key (this is an operator action, not a public endpoint — see `DESIGN.md` section 5 for why):

```bash
docker compose exec api create-api-key "Acme Inc"
# business_id: ...
# api_key:     sk_live_...   <- store this now, it is never shown again
```

Export it for the examples below:

```bash
export TOKEN=sk_live_...
```

## curl examples

**Create a customer:**

```bash
curl -s http://localhost:8080/customers \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"name":"Jane Doe","email":"jane@example.com"}'
```

**Create an invoice** (total is always server-computed from line items, never trusted from the client), then finalize it so it's payable (`draft → open`):

```bash
INVOICE=$(curl -s http://localhost:8080/invoices \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"customer_id":"<customer-id-from-above>","line_items":[{"description":"Widget","quantity":2,"unit_amount_cents":1500}]}')

INVOICE_ID=$(echo "$INVOICE" | grep -oE '"id":"[a-f0-9-]+"' | head -1 | cut -d'"' -f4)

curl -s http://localhost:8080/invoices/$INVOICE_ID/finalize \
  -H "Authorization: Bearer $TOKEN" -X POST
```

**Attempt payment — success case:**

```bash
curl -s http://localhost:8080/invoices/$INVOICE_ID/pay \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{"card_token":"tok_success"}'
# -> 200, {"status":"succeeded", "psp_ref": "...", ...}
```

**Attempt payment — failure case:**

```bash
curl -s http://localhost:8080/invoices/$INVOICE_ID/pay \
  -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{"card_token":"tok_card_declined"}'
# -> 200, {"status":"failed", "failure_code": "card_declined", ...}
```

Every `POST /invoices/{id}/pay` requires a unique `Idempotency-Key` header; retrying the *same* key + body returns the original result without charging again (see `DESIGN.md` section 3).

## API documentation

Full request/response shapes and the error format: [`docs/openapi.yaml`](docs/openapi.yaml).

## Testing

```bash
docker compose up -d postgres mock-psp
DATABASE_URL=postgres://invoice:invoice@localhost:5432/invoice_service \
MOCK_PSP_URL=http://localhost:9090 \
cargo test --test concurrent_payment_test --test idempotency_test --test psp_failure_test
```

These are black-box integration tests: each spins up the real router on an ephemeral port and drives it over real HTTP against the real Postgres and real mock-psp, per the assignment's required coverage:

- **`concurrent_payment_test`** — fires 15 concurrent `POST /pay` at one invoice; asserts exactly one succeeds, the rest are rejected with `409`, and exactly one `payment_attempts` row is `succeeded`.
- **`idempotency_test`** — retries the same `Idempotency-Key` + body and asserts an identical response with no second PSP call (and that reusing the key with a *different* body is rejected); see `DESIGN.md` 3(d).
- **`psp_failure_test`** — covers both `tok_network_error` (attempt marked `failed`, invoice stays payable, a fresh attempt succeeds) and `tok_timeout` (returns `202` well under the mock's 30s sleep, invoice left `open`, never hangs); see `DESIGN.md` 3(b).

No other tests were written beyond these three — the assignment asks to lean on them rather than cover every handler, and the state-machine/money-math logic they exercise is the load-bearing part of this system.

## Demo Video

_TODO: add the Loom/equivalent link here before submitting._

## AI Usage

See [`AI_USAGE.md`](AI_USAGE.md).
