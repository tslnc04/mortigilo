use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::{extract::State, http::StatusCode, routing::get, Router};
use qbit_rs::{
    model::{ConnectionStatus, Credential, GetTorrentListArg, TorrentFilter},
    Qbit,
};
use tracing::{error, info, warn};
use tracing_subscriber::fmt;

const DEFAULT_HOST: &str = "http://localhost:8080";
const DEFAULT_USERNAME: &str = "admin";

const DEFAULT_PORT_STR: &str = "9000";
const DEFAULT_PORT: u16 = 9000;

const DEFAULT_ADDRESS_STR: &str = "0.0.0.0";
const DEFAULT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

#[tokio::main(flavor = "current_thread")]
#[tracing::instrument]
async fn main() -> anyhow::Result<()> {
    let format = fmt::format().with_ansi(false);
    tracing_subscriber::fmt().event_format(format).init();

    let config = Config::from_env()?;
    info!(config.host, config.username, config.port, %config.address);

    let creds = Credential::new(config.username, config.password);
    let qbit = Qbit::new(config.host.as_str(), creds);
    let shared_qbit = Arc::new(qbit);

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(shared_qbit);

    let socket_addr = SocketAddr::new(config.address, config.port);
    let listener = tokio::net::TcpListener::bind(socket_addr).await?;

    info!(address = %socket_addr, "starting server");

    axum::serve(listener, app).await?;

    Ok(())
}

#[tracing::instrument(skip(qbit))]
async fn healthz(State(qbit): State<Arc<Qbit>>) -> StatusCode {
    let resumed_count = match qbit
        .get_torrent_list(GetTorrentListArg {
            filter: Some(TorrentFilter::Resumed),
            ..Default::default()
        })
        .await
    {
        Ok(torrents) if !torrents.is_empty() => torrents.len(),
        Ok(_) => {
            info!("no resumed torrents, responding healthy");
            return StatusCode::NO_CONTENT;
        }
        Err(err) => {
            info!(%err, "unable to get torrent list from qbittorrent, responding not healthy");
            return StatusCode::SERVICE_UNAVAILABLE;
        }
    };

    match qbit
        .get_torrent_list(GetTorrentListArg {
            filter: Some(TorrentFilter::Stalled),
            ..Default::default()
        })
        .await
    {
        Ok(torrents) if torrents.len() >= resumed_count => {
            info!(
                resumed_count,
                stalled_count = torrents.len(),
                "all torrents are stalled, responding not healthy"
            );
            StatusCode::SERVICE_UNAVAILABLE
        }
        Ok(torrents) => {
            info!(
                resumed_count,
                stalled_count = torrents.len(),
                "not all torrents are stalled, responding healthy"
            );
            StatusCode::NO_CONTENT
        }
        Err(err) => {
            info!(%err, "unable to get torrent list from qbittorrent, responding not healthy");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

#[tracing::instrument(skip(qbit))]
async fn readyz(State(qbit): State<Arc<Qbit>>) -> StatusCode {
    let transfer_info = match qbit.get_transfer_info().await {
        Ok(info) => info,
        Err(err) => {
            info!(%err, "unable to get transfer info from qbittorrent, responding not ready");
            return StatusCode::SERVICE_UNAVAILABLE;
        }
    };

    match transfer_info.connection_status {
        ConnectionStatus::Connected | ConnectionStatus::Firewalled => StatusCode::NO_CONTENT,
        status => {
            info!(
                ?status,
                "qbittorrent is not connected or firewalled, responding not ready",
            );
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

#[derive(Debug)]
struct Config {
    host: String,
    username: String,
    password: String,
    port: u16,
    address: IpAddr,
}

impl Config {
    #[tracing::instrument]
    fn from_env() -> anyhow::Result<Self> {
        let host = env_or_default("QBITTORRENT_HOST", DEFAULT_HOST);
        let username = env_or_default("QBITTORRENT_USERNAME", DEFAULT_USERNAME);

        let password = match env::var("QBITTORRENT_PASSWORD") {
            Ok(value) => value,
            Err(_) => {
                error!("qbittorrent password must be provided");
                anyhow::bail!("qbittorrent password must be provided");
            }
        };

        let port = match env_or_default("PORT", DEFAULT_PORT_STR).parse::<u16>() {
            Ok(value) if value > 0 => value,
            Ok(_) => {
                warn!(
                    default = DEFAULT_PORT,
                    "port must not be zero, using default value instead",
                );
                DEFAULT_PORT
            }
            Err(err) => {
                warn!(%err, default = DEFAULT_PORT, "port must be parseable as u16, using default value instead");
                DEFAULT_PORT
            }
        };

        let address = match env_or_default("ADDRESS", DEFAULT_ADDRESS_STR).parse::<IpAddr>() {
            Ok(value) => value,
            Err(err) => {
                warn!(%err, default = DEFAULT_ADDRESS_STR, "address must be parseable as IpAddr, using default value instead");
                DEFAULT_ADDRESS
            }
        };

        Ok(Self {
            host,
            username,
            password,
            port,
            address,
        })
    }
}

fn env_or_default(variable: &str, default: &str) -> String {
    match env::var(variable) {
        Ok(value) => value,
        Err(_) => {
            info!(
                variable,
                default, "environment variable is not set, using default value instead",
            );
            default.to_string()
        }
    }
}
