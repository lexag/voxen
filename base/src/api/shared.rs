use std::net::SocketAddrV4;
use ts_rs::TS;

#[derive(serde::Serialize, serde::Deserialize, TS, Clone)]
#[ts(export)]
pub struct Device {
    pub id: u8,
    pub name: String,
    pub listeners: [Vec<usize>; 3],
    pub ip_addr: SocketAddrV4,
}
