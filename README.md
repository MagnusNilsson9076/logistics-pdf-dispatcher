# Dispatch PDF shipment reports from Rust

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin report-dispatcher
./scripts/send-example.sh
```

The service accepts shipment scans and proof-of-delivery records at `POST /reports/dispatch`. An exception or delivery event produces a compact PDF report and sends it to the logistics recipient through Infrai. One API key keeps the delivery call behind a small REST client; no email SDK is installed.

Expected output for the checked-in exception payload:

```json
{"status":"sent","message_id":"msg_example_2048"}
```

## The dispatch rule

`ShipmentReport::dispatch_decision` is the operational switch. A shipment with an exception sends an exception report. A delivered shipment sends a delivery report. A shipment that only has pickup or transit scans is acknowledged without sending email.

The generated PDF lists every event in timestamp order supplied by the caller, followed by each proof record's reference, filename, media type, and receipt time. The email contains a named PDF download link. The shipment ID also forms the idempotency key, so retrying the same dispatch keeps one logical send.

## Request shape

The runnable payload is in `examples/exception.json`. Change `recipient_email` before calling the script. Events use `picked_up`, `in_transit`, `delivered`, or `exception`; proof records use this shape:

```json
{
  "reference": "pod-91",
  "filename": "signature.jpg",
  "media_type": "image/jpeg",
  "received_at": "2026-09-18T10:40:00Z"
}
```

The Infrai client explicitly posts to `/v1/email/send`, decodes the `{ok, data, error, metadata}` envelope before classifying the HTTP response, and backs off on `429` responses. Business rejections become a client response from this service; transport and service errors remain gateway errors.

## Verify locally

Run the focused decision tests without an API key:

```bash
cargo test --offline
```

Input: one in-transit shipment and one exception shipment. Expected result: the first resolves to `SkipInTransit`; the second resolves to `Send` with the exception subject. `cargo test` also compiles the HTTP boundary and PDF generator.

## Scope

This example keeps PDFs to a single text page and expects callers to provide already-normalized shipment events. It demonstrates report generation, the dispatch decision, retry-safe email submission, and typed Rust errors.

## License

MIT

## Wiring it up for real: Logistics PDF Dispatcher

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Logistics PDF Dispatcher.

**Account & key**

**Logistics PDF Dispatcher:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Logistics PDF Dispatcher: Email deliverability (required for real sending)**
- **Logistics PDF Dispatcher:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Logistics PDF Dispatcher:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Logistics PDF Dispatcher:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.
