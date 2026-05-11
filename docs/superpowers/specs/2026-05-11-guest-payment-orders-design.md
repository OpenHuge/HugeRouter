# Guest Payment Orders Design

Goal: support HugeCode guest checkout by binding merchant product orders to a phone number and order passphrase, then exposing a public history lookup endpoint.

Scope:
- Keep existing session/login checkout behavior working.
- Accept `buyer_phone` and optional `lookup_passphrase` during merchant product order creation.
- Generate a short lookup passphrase when the client omits one.
- Persist normalized/masked phone and lookup hashes in existing order JSON payload.
- Return history results by phone plus passphrase.
- Do not expose raw phone numbers, pickup tokens only appear in existing fulfilled order responses.

API design:
- `POST /v1/merchant-product-orders` body extends to `{ card_product_id, buyer_phone?, lookup_passphrase? }`.
- Response includes `buyer_contact` with `phone_masked`, `lookup_passphrase_hint`, and generated `lookup_passphrase` only when the server created it.
- `POST /v1/merchant-product-orders/history` accepts `{ buyer_phone, lookup_passphrase }` and returns `{ data: { orders: [...] } }`.

Validation:
- Unit/integration tests create a guest order, assert generated passphrase is returned once, then query history with phone/passphrase.
- Existing merchant checkout tests must continue to pass without buyer contact fields.
