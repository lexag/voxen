use crate::{api::web_api, daemon::VoxenDaemon};

mod api;
mod daemon;

#[tokio::main]
async fn main() {
    let mut daemon = VoxenDaemon::new();

    tokio::join!(web_api(daemon.shared_data.clone()));
}
