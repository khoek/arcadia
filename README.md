# Arcadia

Arcadia is a self-contained Google Cloud application for authenticator codes,
passwords, and cards. The repository contains the Svelte frontend and the Rust
API. Its only infrastructure dependency is the public [`khoek/arche`](https://github.com/khoek/arche)
repository; it does not depend on private `vates`, `occultum`, `rete`, `aegis`,
or `caudex` checkouts.

The production shape is:

```text
Google IAP → HTTPS load balancer → Firebase Hosting (frontend)
                              ↘ Cloud Run (API)
```

IAP is the only browser authentication boundary. There is no application OAuth
flow. The API verifies IAP's signed JWT, checks the configured audience, and
admits only the configured Firestore user. The user identity is read from that
user document, so the same image can serve any Google account and project.
Password and card values stay in Firestore; no secret values belong in this
repository.

## Local checks

```sh
npm install
npm run lint
npm run check
npm run build
cargo test --manifest-path api/Cargo.toml
```

## GCP deployment

Create a GCP project with billing, Firestore in Native mode, Firebase Hosting,
Cloud Run, Artifact Registry, and an HTTPS load balancer. Create an IAP OAuth
client in the project, a serverless NEG for the Cloud Run service, and an IAP
backend policy granting `roles/iap.httpsResourceAccessor` to the one Google
account that should use the vault. Configure the load balancer so the frontend
backend serves `/` and the API backend serves `/v2/arcadia/*`.

The frontend can be served at the host root or below a shared host path. For a
path such as `/arcadia`, set `PUBLIC_ARCADIA_BASE=/arcadia` before building and
route that prefix to the Firebase or Cloud Storage frontend. The API remains at
`/v2/arcadia/*` on the same host.

Create the Firestore `v2/arcadia` configuration document with an HTTPS
`origin` and a random `session.pepper`, plus a `v2/auth/users/<id>` document
containing `id`, `email`, `disabled: false`, and `session_version: 0`. The
Each admitted IAP email needs one `v2/auth/users/<id>` document with the same
`email` value. Document IDs are instance-local and may be chosen by the
deployer. Then deploy the API:

```sh
export GOOGLE_CLOUD_PROJECT=your-project
export FIRESTORE_DATABASE_ID=your-firestore-database
export ARCADIA_IAP_AUDIENCE=/projects/PROJECT_NUMBER/global/backendServices/API_BACKEND_SERVICE_ID
./deploy/cloud-run.sh
```

Deploy the frontend to the Firebase Hosting site attached to the load balancer.
The repository intentionally does not select a Firebase site; pass the site ID
chosen by the deployer:

```sh
firebase use your-project
firebase deploy --only hosting:SITE_ID
```

The public repository deliberately leaves project IDs, domains, IAP audiences,
user IDs, OAuth client material, and Firestore data to the deployer.
