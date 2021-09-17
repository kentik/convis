use std::fs;
use std::sync::Arc;
use std::time::SystemTime;
use std::process::exit;
use anyhow::Result;
use env_logger::Builder;
use gumdrop::Options;
use log::{trace, LevelFilter};
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use convis::code::Code;
use convis::data::Record;
use convis::sink::Sink;
use convis::track::Tracker;

#[derive(Options)]
pub struct Args {
    #[options()]
    help: bool,
    #[options()]
    bytecode: Option<String>,
    #[options()]
    sink: Option<Sink>,
    #[options(count)]
    verbose: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_args_default_or_exit();
    let sink = args.sink.unwrap_or_default();

    let mut builder = Builder::from_default_env();
    builder.filter(None, match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    });
    builder.init();

    let mut code = Code::load(&match args.bytecode {
        Some(file) => fs::read(file)?,
        None       => BYTECODE.to_vec(),
    })?;

    let mut sigint  = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::spawn(async move {
        select! {
            _ = sigint.recv()  => exit(0),
            _ = sigterm.recv() => exit(0),
        }
    });

    let hostname = Arc::new(hostname::get()?.to_string_lossy().to_string());
    let tracker  = Arc::new(Tracker::new().await?);

    let (execs, mut socks) = code.exec()?;
    tracker.clone().spawn(execs);

    while let Some(event) = socks.recv().await {
        let timestamp = SystemTime::now();

        trace!("{:?}", event);

        if let Some(process) = tracker.get(event.pid).await {
            let record = Record {
                timestamp: timestamp,
                event:     format!("{:?}", event.call),
                src:       event.src,
                dst:       event.dst,
                process:   process,
                hostname:  hostname.clone(),
                rx:        event.rx,
                tx:        event.tx,
            };
            trace!("{:?}", record);
            sink.send(record)?;
        };
    }

    Ok(())
}

const BYTECODE: &[u8] = include_bytes!("../bpf/bytecode.o");
