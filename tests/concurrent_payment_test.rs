// Fires N concurrent POST /invoices/{id}/pay requests for the same invoice

mod common;

use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

const CONCURRENT_REQUESTS: usize = 15;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn only_one_of_n_concurrent_payments_succeeds() {
    let app = common::spawn_app().await;
    let (_business_id, token) = app.new_business().await;
    let (invoice_id, _total) = app.create_open_invoice(&token, 5_000).await;

    let mut handles = Vec::with_capacity(CONCURRENT_REQUESTS);
    for i in 0..CONCURRENT_REQUESTS {
        let base_url = app.base_url.clone();
        let client = app.client.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            client
                .post(format!("{base_url}/invoices/{invoice_id}/pay"))
                .bearer_auth(&token)
                .header("Idempotency-Key", format!("concurrency-test-{i}-{}", Uuid::new_v4()))
                .json(&serde_json::json!({ "card_token": "tok_success" }))
                .send()
                .await
                .expect("pay request failed")
                .status()
                .as_u16()
        }));
    }

    let mut status_counts: HashMap<u16, u32> = HashMap::new();
    for handle in handles {
        let status = handle.await.expect("pay task panicked");
        *status_counts.entry(status).or_insert(0) += 1;
    }

    assert_eq!(
        status_counts.get(&200).copied().unwrap_or(0),
        1,
        "expected exactly one 200 (succeeded) among {CONCURRENT_REQUESTS} concurrent payments, got: {status_counts:?}"
    );
    assert_eq!(
        status_counts.get(&409).copied().unwrap_or(0),
        (CONCURRENT_REQUESTS - 1) as u32,
        "expected every other request to be rejected with 409 conflict, got: {status_counts:?}"
    );

    let succeeded = app.count_payment_attempts(invoice_id, "succeeded").await;
    assert_eq!(
        succeeded, 1,
        "exactly one payment_attempts row must be 'succeeded' for this invoice -- no double charge"
    );

    let invoice: Value = app.get_invoice(&token, invoice_id).await;
    assert_eq!(invoice["invoice"]["state"], "paid");
}
