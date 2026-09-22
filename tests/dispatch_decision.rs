use logistics_pdf_dispatcher::shipment_report::{
    DispatchDecision, EventKind, ShipmentEvent, ShipmentReport,
};

fn report_with(kind: EventKind) -> ShipmentReport {
    ShipmentReport {
        shipment_id: "SHP-2048".to_owned(),
        recipient_email: "ops@example.com".to_owned(),
        events: vec![ShipmentEvent {
            kind,
            occurred_at: "2026-09-18T08:30:00Z".to_owned(),
            location: "Shanghai hub".to_owned(),
            detail: "carrier scan".to_owned(),
        }],
        proof_of_delivery: vec![],
    }
}

#[test]
fn in_transit_shipments_do_not_emit_a_report_email() {
    let report = report_with(EventKind::InTransit);
    assert_eq!(report.dispatch_decision(), DispatchDecision::SkipInTransit);
}

#[test]
fn exceptions_emit_an_exception_report() {
    let report = report_with(EventKind::Exception);
    assert_eq!(
        report.dispatch_decision(),
        DispatchDecision::Send {
            subject: "Shipment exception report: SHP-2048".to_owned()
        }
    );
}
