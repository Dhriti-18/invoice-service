// Retries the same request with the same Idempotency-Key and asserts the.

mod common;

use serde_json::Value;
use uuid::Uuid;

#[tokio::test]
async fn same_idempotency_key_replays_the_first_result() {
    let app = common::spawn_app().await;
    let (business_id, token) = app.new_business().await;
    let (invoice_id, _total) = app.create_open_invoice(&token, 2_500).await;

    let idempotency_key = format!("idem-test-{}", Uuid::new_v4());

    let first = app.pay(&token, invoice_id, &idempotency_key, "tok_success").await;
    assert_eq!(first.status(), 200);
    let first_body: Value = first.json().await.expect("parse first response");

    // Same key, same body, sent again.
    let second = app.pay(&token, invoice_id, &idempotency_key, "tok_success").await;
    assert_eq!(second.status(), 200);
    let second_body: Value = second.json().await.expect("parse second response");

    assert_eq!(first_body["id"], second_body["id"], "replay must return the same payment attempt");
    assert_eq!(
        first_body["psp_ref"], second_body["psp_ref"],
        "replay must not mint a new psp_ref"
    );
    assert_eq!(second_body["status"], "succeeded");

    let attempts = app.count_payment_attempts_for_key(business_id, &idempotency_key).await;
    assert_eq!(attempts, 1, "exactly one payment_attempts row must exist for this idempotency key");
}

#[tokio::test]
async fn reusing_idempotency_key_with_different_body_is_rejected() {
    let app = common::spawn_app().await;
    let (_business_id, token) = app.new_business().await;
    let (invoice_id, _total) = app.create_open_invoice(&token, 2_500).await;

    let idempotency_key = format!("idem-reuse-test-{}", Uuid::new_v4());

    let first = app.pay(&token, invoice_id, &idempotency_key, "tok_success").await;
    assert_eq!(first.status(), 200);

    let second = app.pay(&token, invoice_id, &idempotency_key, "tok_card_declined").await;
    assert_eq!(second.status(), 409);
    let body: Value = second.json().await.expect("parse rejection body");
    assert_eq!(body["error"]["type"], "idempotency_key_reuse");
}
