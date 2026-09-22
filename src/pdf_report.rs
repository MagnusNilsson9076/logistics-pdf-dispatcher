use crate::shipment_report::ShipmentReport;

pub fn render(report: &ShipmentReport) -> Vec<u8> {
    let mut lines = vec![
        "Logistics shipment report".to_owned(),
        format!("Shipment: {}", report.shipment_id),
        format!("Recipient: {}", report.recipient_email),
        "Events".to_owned(),
    ];
    lines.extend(report.events.iter().map(|event| {
        format!(
            "{} | {:?} | {} | {}",
            event.occurred_at, event.kind, event.location, event.detail
        )
    }));
    lines.push("Proof of delivery".to_owned());
    lines.extend(report.proof_of_delivery.iter().map(|proof| {
        format!(
            "{} | {} | {} | {}",
            proof.received_at, proof.filename, proof.media_type, proof.reference
        )
    }));

    let mut stream = String::from("BT\n/F1 11 Tf\n50 780 Td\n");
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            stream.push_str("0 -16 Td\n");
        }
        stream.push('(');
        stream.push_str(&pdf_escape(line));
        stream.push_str(") Tj\n");
    }
    stream.push_str("ET\n");

    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}endstream", stream.len(), stream),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn pdf_escape(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '(' => "\\(".to_owned(),
            ')' => "\\)".to_owned(),
            '\\' => "\\\\".to_owned(),
            character if character.is_ascii() && !character.is_control() => character.to_string(),
            _ => "?".to_owned(),
        })
        .collect()
}
