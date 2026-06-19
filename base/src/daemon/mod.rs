use crate::api::shared::Device;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};
use tokio::sync::Mutex;

pub struct SharedDaemonData {
    pub devices: Vec<Device>,
    pub local_port: u16,
}

impl Default for SharedDaemonData {
    fn default() -> Self {
        Self {
            local_port: 3000,
            devices: vec![
                Device {
                    id: 1,
                    name: "Alice".to_string(),
                    listeners: Default::default(),
                    ip_addr: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
                },
                Device {
                    id: 2,
                    name: "Bob".to_string(),
                    listeners: Default::default(),
                    ip_addr: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
                },
            ],
        }
    }
}

pub type WrappedDaemonData = Arc<Mutex<SharedDaemonData>>;

pub struct VoxenDaemon {
    pub shared_data: WrappedDaemonData,
}

impl VoxenDaemon {
    pub fn new() -> Self {
        Self {
            shared_data: Arc::new(Mutex::new(SharedDaemonData::default())),
        }
    }
}

fn main() {}
