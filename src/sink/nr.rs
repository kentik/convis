use std::convert::TryInto;
use anyhow::Result;
use reqwest::{Client as HttpClient, Method, Request, Url};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::json;
use crate::data::{Record, Process};
use crate::event::Kind;

pub struct Client {
    client:   HttpClient,
    endpoint: Url,
}

impl Client {
    pub fn new(key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Api-Key", HeaderValue::from_str(key)?);
        headers.insert(CONTENT_TYPE, "application/json".try_into()?);

        let client = HttpClient::builder().default_headers(headers).build()?;
        let endpoint = Url::parse("https://log-api.newrelic.com/log/v1")?;

        Ok(Self { client, endpoint })
    }

    pub async fn send(&self, record: Record) -> Result<()> {
        let Record { process: Process { pid, .. }, dst, .. } = record;

        let source  = env!("CARGO_PKG_NAME");
        let message = match record.event {
            Kind::Connect => format!("{} connected to {}", pid, dst),
            Kind::Close   => format!("{} closed connection to {}", pid, dst),
            Kind::Accept  => format!("{} accept from {}", pid, dst),
        };

        let payload = json!([{
            "common": {
                "attributes": {
                    "source": source,
                },
            },
            "logs": [{
                "attributes": record,
                "message":    message,
            }],
        }]);

        let endpoint = self.endpoint.clone();
        let mut req  = Request::new(Method::POST, endpoint);
        *req.body_mut() = Some(serde_json::to_vec(&payload)?.into());

        self.client.execute(req).await?;

        Ok(())
    }
}
