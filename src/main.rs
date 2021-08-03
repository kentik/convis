use std::fs;
use anyhow::Result;
use env_logger::Builder;
use gumdrop::Options;
use log::{trace, LevelFilter};
use shiplift::Docker;
use convis::code::Code;
use convis::data::Record;
use convis::sink::Sink;

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

    let docker = Docker::new();
    let mut rx = code.exec()?;

    while let Some(event) = rx.recv().await {
        trace!("{:?}", event);

        if let Some(record) = Record::lookup(&docker, event).await? {
            trace!("{:?}", record);
            sink.send(record).await?;
        };
    }

    Ok(())
}

const BYTECODE: &[u8] = include_bytes!("../bpf/bytecode.o");
