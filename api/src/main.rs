mod arcadia;
mod config;
mod endpoint;

mod firestore;
mod path;
mod util;
use axum::{Router, response::IntoResponse, routing::get};
use std::{env, io::Read};

async fn get_root() -> impl IntoResponse {
    "OK"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    arche_web::server::initialize()?;
    let runtime = arche_web::server::ServiceOptions::from_env()?.validate()?;
    if let Some(options) = bootstrap_config_args()? {
        let mut access_token = String::new();
        std::io::stdin().read_to_string(&mut access_token)?;
        let access_token = access_token.trim();
        anyhow::ensure!(
            !access_token.is_empty(),
            "GCP access token is required on stdin"
        );
        let db = arche_firestore::DatabaseOptions {
            project_id: Some(options.project_id),
            credentials: arche_firestore::Credentials::AccessToken(access_token.to_string()),
            ..Default::default()
        }
        .validate()?
        .connect()
        .await?;
        tokio::time::timeout(
            std::time::Duration::from_secs(120),
            firestore::bootstrap_config(&db),
        )
        .await??;
        println!("arcadia-api config bootstrap is complete");
        return Ok(());
    }

    let app = tokio::time::timeout(std::time::Duration::from_secs(120), application()).await??;
    arche_web::server::serve(runtime, app).await
}

async fn application() -> anyhow::Result<Router> {
    let db = arche_firestore::DatabaseOptions::from_env()
        .validate()?
        .connect()
        .await?;
    let cfg = firestore::load_config(&db).await?;
    let iap = endpoint::arcadia::iap::ArcadiaIapAuth::from_env().await?;
    let arcadia_state = endpoint::arcadia::ArcadiaState::new(db, cfg.arcadia, iap)?;

    let api = Router::new()
        .route("/", get(get_root))
        .merge(endpoint::arcadia::router(arcadia_state));

    Ok(Router::new().nest("/v2", api))
}

struct ConfigBootstrapOptions {
    project_id: String,
}

fn bootstrap_config_args() -> anyhow::Result<Option<ConfigBootstrapOptions>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return Ok(None);
    }
    anyhow::ensure!(
        args.len() == 3
            && args[0] == "bootstrap-config"
            && args[1] == "--project-id"
            && !args[2].trim().is_empty(),
        "usage: arcadia-api bootstrap-config --project-id PROJECT"
    );
    Ok(Some(ConfigBootstrapOptions {
        project_id: args[2].clone(),
    }))
}
