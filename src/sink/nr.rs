use std::convert::{TryFrom, TryInto};
use std::mem;
use std::sync::Arc;
use std::time::{UNIX_EPOCH, Duration};
use anyhow::{anyhow, Result};
use flate2::{Compression, write::GzEncoder};
use log::{debug, error, warn};
use parking_lot::Mutex;
use reqwest::{Client as HttpClient, Method, Request, Url};
use reqwest::header::{CONTENT_TYPE, CONTENT_ENCODING, HeaderMap, HeaderValue};
use serde_json::json;
use tokio::time::interval;
use crate::data::Record;
use super::Args;

pub struct Client {
    sender: Arc<Sender>,
}

pub struct Sender {
    client:   HttpClient,
    endpoint: Url,
    records:  Mutex<Vec<Record>>,
}

impl Client {
    pub fn new(args: Args) -> Result<Self> {
        let account = args.get("account")?;
        let key     = args.get("key")?;
        let region  = args.opt("region").unwrap_or("US");

        let host = match region.to_ascii_uppercase().as_str() {
            "US" => "insights-collector.newrelic.com",
            "EU" => "insights-collector.eu01.nr-data.net",
            _    => return Err(anyhow!("invalid region: {}", region)),
        };

        let endpoint = format!("https://{}/v1/accounts/{}/events", host, account);
        let endpoint = Url::parse(&endpoint)?;

        let mut headers = HeaderMap::new();
        headers.insert("X-Insert-Key", HeaderValue::from_str(key)?);
        headers.insert(CONTENT_TYPE, "application/json".try_into()?);
        headers.insert(CONTENT_ENCODING, "gzip".try_into()?);

        let client = HttpClient::builder().default_headers(headers).build()?;
        let sender = Arc::new(Sender::new(client, endpoint));

        let sender2 = sender.clone();
        tokio::spawn(async move {
            match sender2.exec().await {
                Ok(()) => debug!("sender finished"),
                Err(e) => error!("sender failed: {:?}", e),
            }
        });

        Ok(Self { sender })
    }

    pub fn send(&self, record: Record) -> Result<()> {
        self.sender.push(record);
        Ok(())
    }
}

impl Sender {
    fn new(client: HttpClient, endpoint: Url) -> Self {
        let records = Mutex::new(Vec::new());
        Self { client, endpoint, records }
    }

    fn push(&self, record: Record) {
        self.records.lock().push(record);
    }

    fn drain(&self) -> Vec<Record> {
        let mut records = self.records.lock();
        let empty = Vec::with_capacity(records.len());
        mem::replace(&mut records, empty)
    }

    async fn exec(&self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            let payload = self.drain().iter().map(|record| {
                let (id, name, image) = record.process.container.as_ref().map(|c| {
                    (c.id.as_str(), c.name.as_str(), c.image.as_str())
                }).unwrap_or_default();

                let timestamp = record.timestamp.duration_since(UNIX_EPOCH)?;
                let timestamp = u64::try_from(timestamp.as_millis())?;

                Ok(json!({
                    "eventType":        "ContainerVisibility",
                    "timestamp":        timestamp,
                    "event":            &record.event,
                    "source.ip":        record.src.ip(),
                    "source.port":      record.src.port(),
                    "source.host":      &record.hostname,
                    "destination.ip":   record.dst.ip(),
                    "destination.port": record.dst.port(),
                    "process.pid":      record.process.pid,
                    "process.cmd":      &record.process.command.join(" "),
                    "container.id":     id,
                    "container.name":   name,
                    "container.image":  image,
                    "bytes.rx":         record.rx,
                    "bytes.tx":         record.tx,
                }))
            }).collect::<Result<Vec<_>>>()?;

            debug!("sending {} records", payload.len());

            for chunk in payload.chunks(2000) {
                let mut e = GzEncoder::new(Vec::new(), Compression::default());
                serde_json::to_writer(&mut e, chunk)?;
                let body = e.finish()?;

                let endpoint = self.endpoint.clone();
                let mut req  = Request::new(Method::POST, endpoint);
                *req.body_mut() = Some(body.into());

                let res = self.client.execute(req).await?;

                if !res.status().is_success() {
                    let body = res.text().await?;
                    warn!("send failed: {}", body);
                }
            }
        }
    }
}
