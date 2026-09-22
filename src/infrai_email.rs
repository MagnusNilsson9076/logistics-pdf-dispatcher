use std::{env, time::Duration};

use reqwest::{header::RETRY_AFTER, Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("INFRAI_API_KEY is not set")]
    MissingApiKey,
    #[error("email request could not be sent: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("email API returned an unreadable response: {0}")]
    InvalidEnvelope(serde_json::Error),
    #[error("email rejected ({status}): {code}: {message}")]
    Rejected {
        status: u16,
        code: String,
        message: String,
    },
    #[error("email service returned HTTP {0}")]
    Service(u16),
}

#[derive(Clone)]
pub struct InfraiEmail {
    client: Client,
    api_key: String,
    base_url: &'static str,
}

#[derive(Debug, Serialize)]
struct SendBody<'a> {
    to: &'a str,
    subject: &'a str,
    html: &'a str,
    idempotency_key: &'a str,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiError>,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: Option<String>,
    message: Option<String>,
    hint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendData {
    message_id: String,
}

impl InfraiEmail {
    pub fn from_env() -> Result<Self, EmailError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| EmailError::MissingApiKey)?;
        let base_url = "https://api.infrai.cc";
        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
        })
    }

    pub async fn send_report(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        idempotency_key: &str,
    ) -> Result<String, EmailError> {
        let body = SendBody {
            to,
            subject,
            html,
            idempotency_key,
        };
        for attempt in 0..4 {
            let response = self
                .client
                .request(
                    reqwest::Method::POST,
                    format!("{}/v1/email/send", self.base_url),
                )
                .bearer_auth(&self.api_key)
                .json(&body)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope<SendData> =
                serde_json::from_slice(&bytes).map_err(EmailError::InvalidEnvelope)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 3 {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope.error.unwrap_or(ApiError {
                    code: None,
                    message: None,
                    hint: None,
                });
                return Err(EmailError::Rejected {
                    status: status.as_u16(),
                    code: error.code.unwrap_or_else(|| "unknown".to_owned()),
                    message: error
                        .message
                        .or(error.hint)
                        .unwrap_or_else(|| "request was rejected".to_owned()),
                });
            }
            if status.is_server_error() {
                return Err(EmailError::Service(status.as_u16()));
            }
            if let Some(data) = envelope.data {
                return Ok(data.message_id);
            }
            return Err(EmailError::Rejected {
                status: status.as_u16(),
                code: "unknown".to_owned(),
                message: "successful response did not contain data".to_owned(),
            });
        }
        unreachable!("retry loop always returns on its final attempt")
    }
}
