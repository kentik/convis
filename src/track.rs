use std::collections::HashMap;
use std::convert::TryFrom;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Error, Result};
use k8s_cri::v1alpha2::ContainerStatusRequest;
use k8s_cri::v1alpha2::runtime_service_client::RuntimeServiceClient;
use libc::pid_t;
use log::{debug, error};
use parking_lot::{Mutex, RwLock};
use procfs::{process, ProcessCgroup};
use shiplift::Docker;
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use tokio::time::interval;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;
use crate::data::{Container, Process, Status};
use crate::event::Exec;

pub struct Tracker {
    table:  RwLock<HashMap<pid_t, Arc<Process>>>,
    client: Client,
}

struct Client {
    docker: Option<Docker>,
    kube:   Option<Mutex<RuntimeServiceClient<Channel>>>,
}

impl Tracker {
    pub async fn new() -> Result<Self> {
        let client = Client::new().await;
        let table  = RwLock::new(HashMap::new());
        Ok(Self { table, client })
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
        let proc    = process::Process::new(pid).ok()?;
        let command = proc.cmdline().ok()?;
        let cgroups = proc.cgroups().ok()?;
        let status  = Status::Alive;

        let mut container = None;

        for cgroup in &cgroups {
            if let Some(c) = self.client.lookup(cgroup).await {
                container = Some(c);
                break;
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

impl Client {
    async fn new() -> Self {
        let docker = Some(Docker::new());

        let path = "/run/containerd/containerd.sock";
        let kube = async {
            let endpoint = Endpoint::try_from("http://[::]")?;
            let channel  = endpoint.connect_with_connector(service_fn(move |_| {
                UnixStream::connect(path)
            })).await?;
            Ok::<_, Error>(Mutex::new(RuntimeServiceClient::new(channel)))
        }.await.ok();

        Self { docker, kube }
    }

    async fn lookup(&self, cgroup: &ProcessCgroup) -> Option<Container> {
        match cgroup.pathname.split('/').collect::<Vec<_>>()[..] {
            ["", "docker",       id] => self.docker(id).await,
            ["", "kubepods", .., id] => self.kube(id).await,
            _                        => None,
        }
    }

    async fn docker(&self, id: &str) -> Option<Container> {
        let c = self.docker.as_ref()?.containers();
        let c = c.get(id).inspect().await.ok()?;
        Some(Container {
            id:    c.id,
            name:  c.name,
            image: c.config.image,
        })
    }

    async fn kube(&self, id: &str) -> Option<Container> {
        let mut client = self.kube.as_ref()?.lock();

        let s = client.container_status(ContainerStatusRequest {
            container_id: id.to_owned(),
            ..Default::default()
        }).await.ok()?.into_inner().status?;

        Some(Container {
            id:    s.id.clone(),
            name:  s.metadata?.name.clone(),
            image: s.image?.image.clone(),
        })
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
