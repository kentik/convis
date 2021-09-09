use std::str::FromStr;
use anyhow::{Error, Result, anyhow};
use crate::data::Record;
use super::Args;
use super::nr::NewRelicClient;
use super::prom::PrometheusClient;

pub enum Sink {
    NewRelic(NewRelicClient),
    Prometheus(PrometheusClient),
    Stdout,
}

impl Sink {
    pub fn send(&self, record: Record) -> Result<()> {
        match self {
            Self::NewRelic(c)   => c.send(record)?,
            Self::Prometheus(c) => c.send(record)?,
            Self::Stdout        => println!("{:?}", record),
        }
        Ok(())
    }
}

impl Default for Sink {
    fn default() -> Self {
        Self::Stdout
    }
}

impl FromStr for Sink {
   type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        match Args::parse(arg)? {
            ("newrelic",   args) => newrelic(args),
            ("prometheus", args) => prometheus(args),
            ("stdout",    _args) => Ok(Self::Stdout),
            _                    => Err(anyhow!("{}", arg)),
        }
    }
}

fn newrelic(args: Args) -> Result<Sink> {
    Ok(Sink::NewRelic(NewRelicClient::new(args)?))
}

fn prometheus(args: Args) -> Result<Sink> {
    Ok(Sink::Prometheus(PrometheusClient::new(args)?))
}
