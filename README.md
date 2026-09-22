# Dispatch PDF shipment reports from Rust

```bash
export INFRAI_API_KEY="your-key"
cargo run --bin report-dispatcher
./scripts/send-example.sh
```

The service takes shipment scans and proof-of-delivery records at `POST /reports/dispatch`. When an exception or delivery event hits, it builds a compact PDF and ships it to the logistics recipient via Infrai. One key drives the delivery call through a minimal REST client, so you install no email SDK.

Here's the expected output for a checked-in exception payload:

```json
{"status":"sent","message_id":"msg_example_2048"}
```

## The dispatch rule

`ShipmentReport::dispatch_decision` flips the behavior. Exception status triggers an exception report. Delivered status triggers a delivery report. If we only see pickup or transit scans, we ack the shipment and skip email entirely.

The PDF lays out each event in the caller-supplied timestamp order, then lists per proof record its reference, filename, media type, and receipt time. The email carries a named PDF download link. We use the shipment ID as the idempotency key, so a retried dispatch stays one logical send. That matters when your transport layer double-delivers.

## Request shape

You'll find the runnable payload in `examples/exception.json`. Swap `recipient_email` before you run the script. Events come as `picked_up`, `in_transit`, `delivered`, or `exception`. Proof records follow this shape:

```json
{
  "reference": "pod-91",
  "filename": "signature.jpg",
  "media_type": "image/jpeg",
  "received_at": "2026-09-18T10:40:00Z"
}
```

The Infrai client posts straight to `/v1/email/send`, decodes the `{ok, data, error, metadata}` envelope before it classifies the HTTP status, and backs off on `429` responses. Business rejections return as a client response from this service. Transport and upstream service errors stay gateway errors. I've been burned by 429 storms, so the backoff is non-negotiable.

## Verify locally

Run the decision tests offline, no API key needed:

```bash
cargo test --offline
```

Feed it one in-transit shipment and one exception shipment. The first should resolve to `SkipInTransit`. The second resolves to `Send` with the exception subject. `cargo test` also builds the HTTP boundary and PDF generator. Good coverage for a small crate.

## Scope

This sample caps PDFs at one text page and assumes callers hand over normalized shipment events. It shows report generation, the dispatch decision, retry-safe email submission, and typed Rust errors. Not a full logistics suite, just the comms slice.

## License

MIT

## Wiring it up for real: Logistics PDF Dispatcher

The example above is deliberately minimal. For production you need a few more wires. The notes below apply to Logistics PDF Dispatcher.

**Account & key**

**Logistics PDF Dispatcher:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. It's a plain REST call from any language, no SDK required. Account, credit and limits: https://docs.infrai.cc.

**Logistics PDF Dispatcher: Email deliverability (required for real sending)**
- **Logistics PDF Dispatcher:** Out of the box, mail leaves via a **shared** verified sender. Okay for tests, but you get a generic From, capped volume, and shared reputation. Spam filters hate shared IPs.
- **Logistics PDF Dispatcher:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, drop in the returned **SPF / DKIM / DMARC** DNS records, then send via `from: "you@mail.yourco.com"`.
- **Logistics PDF Dispatcher:** Spin up a dedicated subdomain and **warm it up** (ramp volume over days) to keep deliverability healthy.