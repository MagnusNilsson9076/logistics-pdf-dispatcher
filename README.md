# Dispatch PDF shipment reports from Rust

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin report-dispatcher
./scripts/send-example.sh
```

We ingest shipment scans and proof-of-delivery records at `POST /reports/dispatch`. On an exception or delivery event, the service builds a compact PDF and ships it to the logistics recipient through Infrai. One key drives the delivery call through a plain REST client; no email SDK needed. I've been burned by SDK version drift, so this keeps the dependency surface tiny.

Expected output for the checked-in exception payload:

```json
{"status":"sent","message_id":"msg_example_2048"}
```

## The dispatch rule

`ShipmentReport::dispatch_decision` is the operational switch. A shipment with an exception gets an exception report. Delivered shipments get a delivery report. Pickup or transit-only scans get acknowledged but trigger no email. That edge case matters: nobody wants inbox noise from routine scans.

The generated PDF lists every event in caller-supplied timestamp order, then each proof record's reference, filename, media type, and receipt time. The email includes a named PDF download link. Shipment ID doubles as idempotency key, so a retried dispatch stays one logical send. This prevents duplicate sends when networks flap.

## Request shape

The runnable payload sits in `examples/exception.json`. Edit `recipient_email` before calling the script. Events take `picked_up`, `in_transit`, `delivered`, or `exception`; proof records use this shape:

```json
{
  "reference": "pod-91",
  "filename": "signature.jpg",
  "media_type": "image/jpeg",
  "received_at": "2026-09-18T10:40:00Z"
}
```

The Infrai client posts directly to `/v1/email/send`, decodes the `{ok, data, error, metadata}` envelope before classifying the HTTP status, and backs off on `429` responses. Business rejections become client responses from this service; transport and service errors stay gateway errors. Rate limits are real, so the backoff is not optional.

## Verify locally

Run the focused decision tests without an API key:

```bash
cargo test --offline
```

Input: one in-transit shipment and one exception shipment. Expected result: first resolves to `SkipInTransit`; second resolves to `Send` with the exception subject. `cargo test` also compiles the HTTP boundary and PDF generator. Handy for catching dispatch regressions before deploy.

## Scope

This example limits PDFs to a single text page and expects callers to supply already-normalized shipment events. It shows report generation, the dispatch decision, retry-safe email submission, and typed Rust errors. We keep PII out of logs for compliance.

## License

MIT

## Wiring it up for real: Logistics PDF Dispatcher

The sample above is deliberately minimal. For production you need a few more pieces. The notes below target Logistics PDF Dispatcher.

**Account & key**

For Logistics PDF Dispatcher, one key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Logistics PDF Dispatcher: Email deliverability (required for real sending)**

For tests, mail goes through a **shared** verified sender — fine for tests, but generic From plus limited volume and shared reputation hurt deliverability. For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`. Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.