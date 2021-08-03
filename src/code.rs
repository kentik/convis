use std::convert::TryFrom;
use std::future::Future;
use bytes::BytesMut;
use anyhow::Result;
use aya::{Bpf, Btf};
use aya::maps::perf::AsyncPerfEventArray;
use aya::programs::Program;
use aya::util::online_cpus;
use log::{debug, error};
use tokio::sync::mpsc::{channel, Receiver};
use crate::event::Event;

pub struct Code {
    bpf: Bpf,
}

impl Code {
    pub fn load(bytecode: &[u8]) -> Result<Self> {
        let btf = Btf::from_sys_fs().ok();
        let bpf = Bpf::load(&bytecode, btf.as_ref())?;
        Ok(Self { bpf })
    }

    pub fn exec(&mut self) -> Result<Receiver<Event>> {
        let events = self.bpf.map_mut("events")?;

        let mut events = AsyncPerfEventArray::try_from(events)?;
        let (tx, rx) = channel(1024);

        for cpu in online_cpus()? {
            let mut buf  = events.open(cpu, None)?;
            let mut bufs = (0..10).map(|_| {
                BytesMut::with_capacity(1024)
            }).collect::<Vec<_>>();

            let tx = tx.clone();

            spawn(async move {
                loop {
                    let events = buf.read_events(&mut bufs).await?;
                    for i in 0..events.read {
                        let buf = &mut bufs[i];
                        let event = Event::try_from(&buf[..])?;
                        tx.send(event).await?;
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
            }
        }

        Ok(rx)
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
