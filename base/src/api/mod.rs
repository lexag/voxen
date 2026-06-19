use crate::{
    api::shared::Device,
    daemon::{SharedDaemonData, WrappedDaemonData},
};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use protocol::DeviceConfig;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

pub mod shared;

struct DataHolder {
    pub data: WrappedDaemonData,
}

pub async fn web_api(shared_data: WrappedDaemonData) {
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/dist");

    let app = Router::new()
        .route("/api/devices", get(get_devices))
        .route("/api/device/add", post(device_add))
        .fallback_service(ServeDir::new(dist))
        .with_state(shared_data);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    println!("Listening on http://localhost:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn get_devices(State(shared_data): State<WrappedDaemonData>) -> Json<Vec<Device>> {
    let data = shared_data.lock().await;
    Json(data.devices.clone())
}

async fn device_add(
    State(shared_data): State<WrappedDaemonData>,
    Json(new_device): Json<Device>,
) -> Result<String, StatusCode> {
    let mut data = shared_data.lock().await;
    if data
        .devices
        .iter()
        .find(|d| d.id == new_device.id)
        .is_some()
    {
        return Err(StatusCode::CONFLICT);
    }
    data.devices.push(new_device.clone());

    let Ok(local_ip) = local_ip_address::local_ip() else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };
    let IpAddr::V4(addr) = local_ip else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let config = DeviceConfig {
        id: new_device.id,
        base_addr: SocketAddrV4::new(addr, data.local_port),
    };

    config.as_base64().ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn device_add_beltpack() -> impl IntoResponse {}
