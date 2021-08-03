use std::str::FromStr;
use anyhow::{Error, Result, anyhow};
use crate::data::Record;
use super::nr::Client;

pub enum Sink {
    NewRelic(Client),
    Stdout,
}

impl Sink {
    pub async fn send(&self, record: Record) -> Result<()> {
        match self {
            Self::NewRelic(c) => c.send(record).await?,
            Self::Stdout      => println!("{:?}", record),
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s.splitn(2, '=').collect::<Vec<_>>();
        match &args[..] {
            ["newrelic", key] => Ok(Self::NewRelic(Client::new(key)?)),
            _                 => Err(anyhow!("{}", s)),
        }
    }
}
