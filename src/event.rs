use std::convert::{TryFrom, TryInto};
use std::net::{Ipv4Addr, SocketAddr};
use anyhow::{anyhow, Error};
use bytemuck::{try_from_bytes, Pod, Zeroable};
use libc::pid_t;
use serde::Serialize;

#[derive(Debug)]
pub struct Event {
    pub kind: Kind,
    pub pid:  pid_t,
    pub src:  SocketAddr,
    pub dst:  SocketAddr,
}

#[derive(Debug, Serialize)]
pub enum Kind {
    Accept,
    Close,
    Connect,
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
struct Raw {
    event: u32,
    pid:   u32,
    proto: u32,
    saddr: u32,
    sport: u32,
    daddr: u32,
    dport: u32,
}

impl TryFrom<&[u8]> for Event {
    type Error = Error;

    fn try_from(buf: &[u8]) -> Result<Self, Self::Error> {
        let raw = try_from_bytes::<Raw>(buf).map_err(|e| {
            anyhow!("invalid raw event: {}", e)
        })?;

        let saddr = Ipv4Addr::from(raw.saddr.to_be());
        let sport = u16::try_from(raw.sport)?;
        let daddr = Ipv4Addr::from(raw.daddr.to_be());
        let dport = u16::try_from(raw.dport)?;

        let kind = match raw.event {
            1 => Kind::Connect,
            2 => Kind::Accept,
            5 => Kind::Close,
            n => return Err(anyhow!("invalid event: {}", n)),
        };

        Ok(Self {
            kind: kind,
            pid:  raw.pid.try_into()?,
            src:  SocketAddr::new(saddr.into(), sport),
            dst:  SocketAddr::new(daddr.into(), dport),
        })
    }
}
