use std::convert::TryFrom;
use std::future::Future;
use bytes::BytesMut;
use anyhow::Result;
use aya::Bpf;
use aya::maps::perf::AsyncPerfEventArray;
use aya::programs::Program;
use aya::util::online_cpus;
use log::{debug, error};
use tokio::sync::mpsc::{channel, Receiver};
use crate::event::{Event, Exec, Sock};

pub struct Code {
    bpf: Bpf,
}

impl Code {
    pub fn load(bytecode: &[u8]) -> Result<Self> {
        let bpf = Bpf::load(&bytecode)?;
        Ok(Self { bpf })
    }

    pub fn exec(&mut self) -> Result<(Receiver<Exec>, Receiver<Sock>)> {
        let events = self.bpf.map_mut("events")?;

        let mut events = AsyncPerfEventArray::try_from(events)?;
        let (tx0, rx0) = channel(1024);
        let (tx1, rx1) = channel(1024);

        for cpu in online_cpus()? {
            let mut buf  = events.open(cpu, None)?;
            let mut bufs = (0..10).map(|_| {
                BytesMut::with_capacity(1024)
            }).collect::<Vec<_>>();

            let tx0 = tx0.clone();
            let tx1 = tx1.clone();

            spawn(async move {
                loop {
                    let events = buf.read_events(&mut bufs).await?;
                    for buf in bufs.iter_mut().take(events.read) {
                        match Event::read(&buf[..]) {
                            Ok(Event::Exec(e)) => tx0.send(e).await?,
                            Ok(Event::Sock(s)) => tx1.send(s).await?,
                            Err(e)             => error!("{}", e),
                        };
                    }
                }
            });
        }

        let names = self.bpf.programs().map(|p| {
            p.name().to_owned()
        }).collect::<Vec<_>>();

        for name in names {
            let prog = self.bpf.program_mut(&name)?;
            prog.load()?;

            debug!("loaded {}", name);

            if let Program::KProbe(kprobe) = prog {
                let func = match name.as_str() {
                    "call-tcp-connect" => "tcp_v4_connect",
                    "exit-tcp-connect" => "tcp_v4_connect",
                    name               => name,
                };
                kprobe.attach(func, 0)?;
            } else if let Program::TracePoint(trace) = prog {
                let index = name.find('/').unwrap_or(0);
                let (category, name) = name.split_at(index);
                trace.attach(category, name)?;
            }
        }

        Ok((rx0, rx1))
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
