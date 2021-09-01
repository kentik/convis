use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use libc::pid_t;
use log::{debug, error};
use parking_lot::RwLock;
use shiplift::Docker;
use tokio::sync::mpsc::Receiver;
use tokio::time::interval;
use crate::data::{Container, Process, Status};
use crate::event::Exec;

#[derive(Default)]
pub struct Tracker {
    table:  RwLock<HashMap<pid_t, Arc<Process>>>,
    docker: Docker,
}

impl Tracker {
    pub fn new(docker: Docker) -> Self {
        let table = RwLock::new(HashMap::new());
        Self { table, docker }
    }

    pub fn spawn(self: Arc<Self>, rx: Receiver<Exec>) {
        spawn(self.clone().recv(rx));
        spawn(self.sweep());
    }

    pub async fn get(&self, pid: pid_t) -> Option<Arc<Process>> {
        let process = self.table.read().get(&pid).cloned();

        if let Some(process) = process {
            return Some(process);
        }

        let process = self.lookup(pid).await?;
        self.table.write().insert(pid, process)
    }

    async fn recv(self: Arc<Self>, mut rx: Receiver<Exec>) -> Result<()> {
        while let Some(e) = rx.recv().await {
            match e {
                Exec::Exec(pid) => self.exec(pid).await,
                Exec::Exit(pid) => self.exit(pid).await,
            }
        }
        Ok(())
    }

    async fn exec(&self, pid: pid_t) {
        if let Some(process) = self.lookup(pid).await {
            self.table.write().insert(pid, process);
        }
    }

    async fn exit(&self, pid: pid_t) {
        self.table.write().entry(pid).and_modify(|p| {
            *p = Arc::new(Process {
                pid:       p.pid,
                command:   p.command.clone(),
                container: p.container.clone(),
                status:    Status::Dead,
            });
        });
    }

    async fn lookup(&self, pid: pid_t) -> Option<Arc<Process>> {
        let proc    = procfs::process::Process::new(pid).ok()?;
        let command = proc.cmdline().ok()?;
        let cgroups = proc.cgroups().ok()?;
        let status  = Status::Alive;

        let mut container = None;

        for cgroup in cgroups {
            let path = cgroup.pathname.split('/').collect::<Vec<_>>();
            if let ["", "docker", id] = path[..] {
                let c = self.docker.containers().get(id);
                if let Ok(c) = c.inspect().await {
                    container = Some(Container {
                        id:    c.id,
                        name:  c.name,
                        image: c.config.image,
                    });
                    break;
                }
            }
        }

        Some(Arc::new(Process { pid, command, container, status }))
     }

    async fn sweep(self: Arc<Self>) -> Result<()> {
        let mut interval = interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let mut table = self.table.write();
            let n = table.len();

            table.retain(|_, p| p.status == Status::Alive);

            debug!("swept {} dead processes", n - table.len());
        }
    }
}

fn spawn<F: Future<Output = Result<()>> + Send + 'static>(task: F) {
    tokio::spawn(async move {
        match task.await {
            Ok(()) => debug!("task finished"),
            Err(e) => error!("task failed: {:?}", e),
        }
    });
}
