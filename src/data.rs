use std::net::SocketAddr;
use anyhow::Result;
use libc::pid_t;
use procfs::ProcessCgroup;
use serde::Serialize;
use shiplift::Docker;
use crate::event::{Event, Kind};

#[derive(Debug, Serialize)]
pub struct Record {
    pub event:     Kind,
    pub src:       SocketAddr,
    pub dst:       SocketAddr,
    pub process:   Process,
    pub container: Option<Container>,
    pub hostname:  String,
}

#[derive(Debug, Serialize)]
pub struct Process {
    pub pid: pid_t,
    pub cmd: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Container {
    pub id:    String,
    pub name:  String,
    pub image: String,
}

impl Record {
    pub async fn lookup(docker: &Docker, event: Event) -> Result<Option<Record>> {
        let Event { kind: event, pid, src, dst, .. } = event;

        let (process, cgroups) = match lookup(pid) {
            Some((p, cg)) => (p, cg),
            None          => return Ok(None),
        };

        let mut container = None;

        for cgroup in cgroups {
            let path = cgroup.pathname.split('/').collect::<Vec<_>>();
            if let ["", "docker", id] = path[..] {
                let c = docker.containers().get(id);
                let c = c.inspect().await?;
                container = Some(Container {
                    id:    c.id,
                    name:  c.name,
                    image: c.config.image,
                });
                break;
            }
        }

        let hostname = hostname::get()?.to_string_lossy().to_string();

        Ok(Some(Self { event, src, dst, process, container, hostname }))
    }
}

fn lookup(pid: pid_t) -> Option<(Process, Vec<ProcessCgroup>)> {
    let proc    = procfs::process::Process::new(pid).ok()?;
    let cmd     = proc.cmdline().ok()?;
    let cgroups = proc.cgroups().ok()?;
    Some((Process { pid, cmd }, cgroups))
}
