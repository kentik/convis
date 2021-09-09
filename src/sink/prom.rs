use std::convert::{TryFrom, TryInto};
use std::io::Write;
use std::mem;
use std::sync::Arc;
use std::time::{UNIX_EPOCH, Duration};
use anyhow::Result;
use base64::{STANDARD, write::EncoderStringWriter};
use log::{debug, error, warn};
use parking_lot::Mutex;
use prost::Message;
use reqwest::{Client as HttpClient, Method, Request, Url};
use reqwest::header::{self, HeaderMap, HeaderValue};
use snap::raw::Encoder;
use tokio::time::interval;
use crate::data::Record;
use super::Args;

pub struct PrometheusClient {
    sender: Arc<Sender>,
}

struct Sender {
    client:   HttpClient,
    endpoint: Url,
    records:  Mutex<Vec<Record>>,
}

impl PrometheusClient {
    pub fn new(args: Args) -> Result<Self> {
        let endpoint = args.get("endpoint")?;
        let username = args.opt("username");
        let password = args.opt("password");

        let endpoint = Url::parse(&endpoint)?;

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, "snappy".try_into()?);
        headers.insert(header::CONTENT_TYPE, "application/x-protobuf".try_into()?);
        headers.insert(header::USER_AGENT, env!("CARGO_PKG_NAME").try_into()?);
        headers.insert("X-Prometheus-Remote-Write-Version", "0.1.0".try_into()?);

        if let Some((username, password)) = username.zip(password) {
            let mut buf = "Basic ".to_string();
            let mut enc = EncoderStringWriter::from(&mut buf, STANDARD);
            write!(enc, "{}:{}", username, password)?;
            let value = HeaderValue::from_str(&enc.into_inner())?;
            headers.insert(header::AUTHORIZATION, value);
        }

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

#[derive(Message)]
pub struct WriteRequest {
    #[prost(message, repeated)]
    pub timeseries: Vec<TimeSeries>,
}

#[derive(Message)]
pub struct TimeSeries {
    #[prost(message, repeated)]
    pub labels:  Vec<Label>,
    #[prost(message, repeated)]
    pub samples: Vec<Sample>,
}

#[derive(Message, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Label {
    #[prost(string)]
    pub name:  String,
    #[prost(string)]
    pub value: String,
}

#[derive(Message)]
pub struct Sample {
    #[prost(double)]
    pub value:     f64,
    #[prost(int64)]
    pub timestamp: i64,
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

            let records    = self.drain();
            let mut series = Vec::with_capacity(records.len() * 2);

            for record in records {
                let timestamp = record.timestamp.duration_since(UNIX_EPOCH)?;
                let secs = i64::try_from(timestamp.as_secs())? * 1000;
                let ms   = i64::try_from(timestamp.subsec_millis())?;
                let timestamp = secs + ms;

                let mut labels = Vec::new();
                let mut label  = |name: &str, value: String| {
                    labels.push(Label {
                        name:  name.to_owned(),
                        value: value,
                    })
                };

                label("event",            record.event);
                label("source_ip",        record.src.ip().to_string());
                label("source_port",      record.src.port().to_string());
                label("source_host",      record.hostname.to_string());
                label("destination_ip",   record.dst.ip().to_string());
                label("destination_port", record.dst.port().to_string());
                label("process_pid",      record.process.pid.to_string());
                label("process_cmd",      record.process.command.join(" "));

                if let Some(container) = &record.process.container {
                    label("container_id",    container.id.to_string());
                    label("container_name",  container.name.to_string());
                    label("container_image", container.image.to_string());
                }

                let mut labels0 = labels.clone();
                labels0.push(Label {
                    name:  "__name__".to_owned(),
                    value: "bytes_rx".to_owned(),
                });
                labels0.sort_unstable();

                let mut labels1 = labels;
                labels1.push(Label {
                    name:  "__name__".to_owned(),
                    value: "bytes_tx".to_owned(),
                });
                labels1.sort_unstable();

                series.push(TimeSeries {
                    labels: labels0,
                    samples: vec![Sample {
                        value:     record.rx.try_into()?,
                        timestamp: timestamp,
                    }],
                });

                series.push(TimeSeries {
                    labels: labels1,
                    samples: vec![Sample {
                        value:     record.rx.try_into()?,
                        timestamp: timestamp,
                    }],
                });
            }

            debug!("sending {} records", series.len());

            let mut buf = Vec::new();
            WriteRequest {
                timeseries: series,
            }.encode(&mut buf)?;

            let body = Encoder::new().compress_vec(&buf)?;

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
