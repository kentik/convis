use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use libc::pid_t;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Process {
    pub pid:       pid_t,
    pub command:   Vec<String>,
    pub container: Option<Container>,
    pub pod:       Option<Pod>,
    pub status:    Status,
}

#[derive(Clone, Debug, Serialize)]
pub struct Container {
    pub id:     String,
    pub name:   String,
    pub image:  String,
    pub labels: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Pod {
    pub name: String,
    pub namespace: String,
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
    pub rx:        u32,
    pub tx:        u32,
    pub srtt:      Duration,
    pub retx:      u32,
}
