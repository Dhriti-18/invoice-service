mod common;

use std::time::{Duration, Instant};

use serde_json::Value;

#[tokio::test]
async fn network_error_marks_the_attempt_failed_and_invoice_stays_payable() {
    let app = common::spawn_app().await;
    let (_business_id, token) = app.new_business().await;
    let (invoice_id, _total) = app.create_open_invoice(&token, 1_000).await;

    let first_key = format!("psp-fail-test-{}", uuid::Uuid::new_v4());
    let resp = app.pay(&token, invoice_id, &first_key, "tok_network_error").await;
    assert_eq!(
        resp.status(),
        200,
        "an ambiguous PSP error that we resolve to 'failed' is a 200 with a failed attempt, not a 5xx"
    );
    let body: Value = resp.json().await.expect("parse response");
    assert_eq!(body["status"], "failed");
    assert_eq!(body["failure_code"], "network_error");

    let invoice: Value = app.get_invoice(&token, invoice_id).await;
    assert_eq!(invoice["invoice"]["state"], "open", "invoice must stay payable, not get stuck");

    let retry_key = format!("psp-fail-retry-{}", uuid::Uuid::new_v4());
    let retry = app.pay(&token, invoice_id, &retry_key, "tok_success").await;
    assert_eq!(retry.status(), 200);
    let retry_body: Value = retry.json().await.expect("parse retry response");
    assert_eq!(retry_body["status"], "succeeded");

    let invoice: Value = app.get_invoice(&token, invoice_id).await;
    assert_eq!(invoice["invoice"]["state"], "paid");
}

#[tokio::test]
async fn timeout_does_not_hang_and_leaves_the_attempt_pending() {
    let app = common::spawn_app().await;
    let (_business_id, token) = app.new_business().await;
    let (invoice_id, _total) = app.create_open_invoice(&token, 1_000).await;

    let key = format!("psp-timeout-test-{}", uuid::Uuid::new_v4());

    let started = Instant::now();

    let resp = tokio::time::timeout(
        Duration::from_secs(10),
        app.pay(&token, invoice_id, &key, "tok_timeout"),
    )
    .await
    .expect("pay endpoint hung past the 10s test timeout instead of returning");
    let elapsed = started.elapsed();

    assert_eq!(
        resp.status(),
        202,
        "an unresolved PSP call must surface as 202 Accepted, not hang or claim success"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "endpoint must return well before the mock PSP's 30s tok_timeout sleep, took {elapsed:?}"
    );

    let body: Value = resp.json().await.expect("parse response");
    assert_eq!(body["status"], "pending");

    let invoice: Value = app.get_invoice(&token, invoice_id).await;
    assert_eq!(
        invoice["invoice"]["state"], "open",
        "invoice must not be flipped to paid on an unresolved result"
    );
}
