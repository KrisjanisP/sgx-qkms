#!/bin/bash
#
# Demonstrates the attestation-bound certificate enrollment flow.
#
# Prerequisites:
#   - CA certificate and key in certs/ca/ca.crt and certs/ca/ca.key
#   - Server certificate (signed by the CA) for the enrollment service
#
# Terminal 1: Start the enrollment service (RA/CA)
#   cargo run -- enroll-service --interactive \
#     --ca-cert certs/ca/ca.crt \
#     --ca-key certs/ca/ca.key \
#     --addr 0.0.0.0:8444
#
#   (without --interactive, approve manually via curl)
#
# Terminal 2: Run this script to enroll a node
#   bash examples/enroll.sh
#

set -e

NODE_ID="${1:-kme-node-1}"
RA_HOST="${2:-localhost}"
RA_PORT="${3:-8444}"
OUT_CERT="enrolled-${NODE_ID}.crt"
OUT_KEY="enrolled-${NODE_ID}.key"

echo "=== Enrollment client ==="
echo "Node ID:  $NODE_ID"
echo "RA:       $RA_HOST:$RA_PORT"
echo "Out cert: $OUT_CERT"
echo "Out key:  $OUT_KEY"
echo ""

# Step 1: Start enrollment (generates key, builds CSR, submits to RA)
# This will block polling until approved.
# In another terminal, approve the enrollment request.
cargo run -- enroll \
  --node-id "$NODE_ID" \
  --ra-host "$RA_HOST" \
  --ra-port "$RA_PORT" \
  --out-cert "$OUT_CERT" \
  --out-key "$OUT_KEY" &
ENROLL_PID=$!

# Give the client time to submit the CSR
sleep 3

echo ""
echo "=== Approving enrollment (admin action) ==="
echo "Fetching pending enrollment ID..."

# The enrollment service runs HTTPS. Use curl with the CA cert to approve.
# First, we need the enrollment ID. The enroll client printed it, but we
# can also discover it from the service logs.
#
# For this demo, manually approve via curl:
#   ENROLL_ID=<id from enrollment service output>
#   curl --cacert certs/ca/ca.crt \
#        -X POST "https://${RA_HOST}:${RA_PORT}/enroll/${ENROLL_ID}/approve"
#
echo "Check the enrollment service output for the enrollment ID, then run:"
echo ""
echo "  curl --cacert certs/ca/ca.crt -X POST https://${RA_HOST}:${RA_PORT}/enroll/<ID>/approve"
echo ""
echo "The enrollment client (PID $ENROLL_PID) is polling and will complete after approval."

wait $ENROLL_PID
echo ""
echo "=== Enrollment complete ==="
echo "Certificate: $OUT_CERT"
echo "Private key: $OUT_KEY"
