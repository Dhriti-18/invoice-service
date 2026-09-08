use std::time::Duration;

use serde::Deserialize;

const PSP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum PspOutcome {
    Succeeded { psp_ref: String },
    Failed { code: String },
}

#[derive(Debug)]
pub enum PspError {
    Timeout,

    NetworkError(String),
}

#[derive(Deserialize)]
struct ChargeResponse {
    status: String,
    psp_ref: Option<String>,
    code: Option<String>,
}

pub async fn charge(
    client: &reqwest::Client,
    base_url: &str,
    card_token: &str,
) -> Result<PspOutcome, PspError> {
    let response = client
        .post(format!("{base_url}/charges"))
        .json(&serde_json::json!({ "card_token": card_token }))
        .timeout(PSP_TIMEOUT)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                PspError::Timeout
            } else {
                PspError::NetworkError(e.to_string())
            }
        })?;

    if !response.status().is_success() {
        return Err(PspError::NetworkError(format!(
            "psp returned status {}",
            response.status()
        )));
    }

    let parsed: ChargeResponse = response
        .json()
        .await
        .map_err(|e| PspError::NetworkError(format!("unparseable psp response: {e}")))?;

    match parsed.status.as_str() {
        "succeeded" => Ok(PspOutcome::Succeeded {
            psp_ref: parsed.psp_ref.unwrap_or_default(),
        }),
        "failed" => Ok(PspOutcome::Failed {
            code: parsed.code.unwrap_or_else(|| "unknown".to_string()),
        }),
        other => Err(PspError::NetworkError(format!(
            "unexpected psp status '{other}'"
        ))),
    }
}
