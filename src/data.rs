use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use libc::pid_t;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Process {
    pub pid:       pid_t,
    pub command:   Vec<String>,
    pub container: Option<Container>,
    pub status:    Status,
}

#[derive(Clone, Debug, Serialize)]
pub struct Container {
    pub id:    String,
    pub name:  String,
    pub image: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum Status {
    Alive,
    Dead,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub timestamp: SystemTime,
    pub event:     String,
    pub src:       SocketAddr,
    pub dst:       SocketAddr,
    pub process:   Arc<Process>,
    pub hostname:  Arc<String>,
}
