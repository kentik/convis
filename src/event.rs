use std::convert::{TryFrom, TryInto};
use std::mem::size_of;
use std::net::{Ipv4Addr, SocketAddr};
use anyhow::{anyhow, Result};
use bytemuck::{try_from_bytes, Pod, Zeroable};
use libc::pid_t;

#[derive(Debug)]
pub enum Event {
    Exec(Exec),
    Sock(Sock),
}

#[derive(Debug)]
pub enum Exec {
    Exec(pid_t),
    Exit(pid_t),
}

#[derive(Debug)]
pub struct Sock {
    pub call: Call,
    pub pid:  pid_t,
    pub src:  SocketAddr,
    pub dst:  SocketAddr,
    pub rx:   u32,
    pub tx:   u32,
    pub srtt: u32,
    pub retx: u32,
}

#[derive(Debug)]
pub enum Call {
    Accept,
    Close,
    Connect,
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
struct Header {
    kind: u32,
    pid:  u32,
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
struct Sock4 {
    proto: u32,
    saddr: u32,
    sport: u32,
    daddr: u32,
    dport: u32,
}

const EXEC:    u32 = 0;
const EXIT:    u32 = 1;
const CONNECT: u32 = 2;
const ACCEPT:  u32 = 3;
const CLOSE:   u32 = 4;

impl Event {
    pub fn read(buf: &[u8]) -> Result<Self> {
        let (head, data) = buf.split_at(size_of::<Header>());

        let head = try_from_bytes::<Header>(head).map_err(|e| {
            anyhow!("invalid header: {}", e)
        })?;

        let pid = head.pid.try_into()?;

        Ok(match head.kind {
            EXEC    => Event::Exec(Exec::Exec(pid)),
            EXIT    => Event::Exec(Exec::Exit(pid)),
            ACCEPT  => Event::Sock(Sock::read(Call::Accept, pid, data)?),
            CLOSE   => Event::Sock(Sock::read(Call::Close, pid, data)?),
            CONNECT => Event::Sock(Sock::read(Call::Connect, pid, data)?),
            kind    => return Err(anyhow!("invalid event: {}", kind)),
        })
    }
}

impl Sock {
    pub fn read(call: Call, pid: pid_t, buf: &[u8]) -> Result<Self> {
        let (data, tail) = buf.split_at(size_of::<Sock4>());

        let sock4 = try_from_bytes::<Sock4>(data).map_err(|e| {
            anyhow!("invalid sock4: {}", e)
        })?;

        let saddr = Ipv4Addr::from(sock4.saddr.to_be());
        let sport = u16::try_from(sock4.sport)?;
        let daddr = Ipv4Addr::from(sock4.daddr.to_be());
        let dport = u16::try_from(sock4.dport)?;

        let src = SocketAddr::new(saddr.into(), sport);
        let dst = SocketAddr::new(daddr.into(), dport);

        let mut rx   = 0;
        let mut tx   = 0;
        let mut srtt = 0;
        let mut retx = 0;

        if tail.len() == 16 {
            rx   = u32::from_ne_bytes(tail[0..4].try_into()?);
            tx   = u32::from_ne_bytes(tail[4..8].try_into()?);
            srtt = u32::from_ne_bytes(tail[8..12].try_into()?);
            retx = u32::from_ne_bytes(tail[12..16].try_into()?);
        }

        Ok(Sock { call, pid, src, dst, rx, tx, srtt, retx })
    }

}
