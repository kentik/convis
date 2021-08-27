use std::collections::HashMap;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Args {
    args: HashMap<String, String>,
}

impl Args {
    pub fn parse(args: &str) -> Result<(&str, Self)> {
        let mut split = args.splitn(2, ',');
        let sink = split.next().unwrap_or("");
        let rest = split.next().unwrap_or("");

        let args = rest.split(',').flat_map(|str| {
            let (k, v) = str.split_once('=')?;
            Some((k.to_owned(), v.to_owned()))
        }).collect::<HashMap<_, _>>();

        Ok((sink, Self { args }))
    }

    pub fn get(&self, name: &str) -> Result<&str> {
        match self.args.get(name) {
            Some(value) => Ok(value.as_str()),
            None        => Err(anyhow!("missing arg '{}'", name)),
        }
    }

    pub fn opt(&self, name: &str) -> Option<&str> {
        self.args.get(name).map(String::as_str)
    }
}
