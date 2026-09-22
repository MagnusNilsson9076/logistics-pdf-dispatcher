use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    PickedUp,
    InTransit,
    Delivered,
    Exception,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShipmentEvent {
    pub kind: EventKind,
    pub occurred_at: String,
    pub location: String,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofOfDelivery {
    pub reference: String,
    pub filename: String,
    pub media_type: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShipmentReport {
    pub shipment_id: String,
    pub recipient_email: String,
    pub events: Vec<ShipmentEvent>,
    pub proof_of_delivery: Vec<ProofOfDelivery>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchDecision {
    Send { subject: String },
    SkipInTransit,
}

impl ShipmentReport {
    pub fn dispatch_decision(&self) -> DispatchDecision {
        let has_exception = self
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Exception));
        if has_exception {
            return DispatchDecision::Send {
                subject: format!("Shipment exception report: {}", self.shipment_id),
            };
        }

        let delivered = self
            .events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Delivered));
        if delivered {
            DispatchDecision::Send {
                subject: format!("Delivery report: {}", self.shipment_id),
            }
        } else {
            DispatchDecision::SkipInTransit
        }
    }
}
