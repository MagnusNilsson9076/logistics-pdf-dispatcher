use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use logistics_pdf_dispatcher::{
    infrai_email::{EmailError, InfraiEmail},
    pdf_report,
    shipment_report::{DispatchDecision, ShipmentReport},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Clone)]
struct AppState {
    email: InfraiEmail,
}

#[derive(Debug, Error)]
enum DispatchError {
    #[error(transparent)]
    Email(#[from] EmailError),
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum DispatchReply {
    Sent { message_id: String },
    Skipped { reason: &'static str },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState {
        email: InfraiEmail::from_env()?,
    });
    let app = Router::new()
        .route("/reports/dispatch", post(dispatch_report))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("report dispatcher listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn dispatch_report(
    State(state): State<Arc<AppState>>,
    Json(report): Json<ShipmentReport>,
) -> Result<Json<DispatchReply>, DispatchError> {
    let subject = match report.dispatch_decision() {
        DispatchDecision::SkipInTransit => {
            return Ok(Json(DispatchReply::Skipped {
                reason: "shipment remains in transit",
            }))
        }
        DispatchDecision::Send { subject } => subject,
    };

    let pdf = pdf_report::render(&report);
    let encoded = STANDARD.encode(pdf);
    let html = format!(
        "<p>Shipment <strong>{}</strong> is ready for review.</p><p><a download=\"shipment-{}.pdf\" href=\"data:application/pdf;base64,{}\">Download the PDF report</a></p>",
        report.shipment_id, report.shipment_id, encoded
    );
    let idempotency_key = format!("shipment-report:{}", report.shipment_id);
    let message_id = state
        .email
        .send_report(&report.recipient_email, &subject, &html, &idempotency_key)
        .await?;
    Ok(Json(DispatchReply::Sent { message_id }))
}

impl IntoResponse for DispatchError {
    fn into_response(self) -> Response {
        let status = match &self {
            DispatchError::Email(EmailError::Rejected { status, .. }) if *status < 500 => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            DispatchError::Email(EmailError::MissingApiKey) => StatusCode::INTERNAL_SERVER_ERROR,
            DispatchError::Email(_) => StatusCode::BAD_GATEWAY,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}
