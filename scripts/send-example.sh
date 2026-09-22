#!/bin/sh
set -eu

curl --fail-with-body \
  --request POST \
  --header 'Content-Type: application/json' \
  --data @examples/exception.json \
  http://127.0.0.1:3000/reports/dispatch

