#!/usr/bin/env bash
set -euo pipefail

: "${GOOGLE_CLOUD_PROJECT:?Set GOOGLE_CLOUD_PROJECT}"
: "${ARCADIA_REGION:=us-central1}"
: "${ARCADIA_SERVICE:=arcadia-api}"
: "${ARCADIA_IAP_AUDIENCE:?Set ARCADIA_IAP_AUDIENCE after creating the IAP API backend}"

gcloud run deploy "${ARCADIA_SERVICE}" \
  --project="${GOOGLE_CLOUD_PROJECT}" \
  --region="${ARCADIA_REGION}" \
  --source=. \
  --platform=managed \
  --allow-unauthenticated \
  --ingress=all \
  --port=8080 \
  --set-env-vars="ARCADIA_IAP_AUDIENCE=${ARCADIA_IAP_AUDIENCE}" \
  --quiet
