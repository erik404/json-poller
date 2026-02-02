use std::error::Error;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use tokio::time::interval;

pub const POLL_INTERVAL_MS: u64 = 500;
pub const POOL_MAX_IDLE_PER_HOST: usize = 1;
pub const POOL_IDLE_TIMEOUT_SECS: u64 = 90;
pub const REQUEST_TIMEOUT_MS: u64 = 1000;
pub const TCP_KEEPALIVE_SECS: u64 = 60;

pub struct JsonPoller<T> {
    client: Client,
    url: String,
    poll_interval: Duration,
    _phantom: PhantomData<T>,
}

pub struct JsonPollerBuilder<T> {
    url: String,
    poll_interval_ms: u64,
    pool_max_idle_per_host: usize,
    pool_idle_timeout_secs: u64,
    request_timeout_ms: u64,
    tcp_keepalive_secs: u64,
    _phantom: PhantomData<T>,
}

impl<T> JsonPollerBuilder<T> {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            poll_interval_ms: POLL_INTERVAL_MS,
            pool_max_idle_per_host: POOL_MAX_IDLE_PER_HOST,
            pool_idle_timeout_secs: POOL_IDLE_TIMEOUT_SECS,
            request_timeout_ms: REQUEST_TIMEOUT_MS,
            tcp_keepalive_secs: TCP_KEEPALIVE_SECS,
            _phantom: PhantomData,
        }
    }

    pub fn poll_interval_ms(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.pool_max_idle_per_host = max;
        self
    }

    pub fn pool_idle_timeout_secs(mut self, secs: u64) -> Self {
        self.pool_idle_timeout_secs = secs;
        self
    }

    pub fn request_timeout_ms(mut self, ms: u64) -> Self {
        self.request_timeout_ms = ms;
        self
    }

    pub fn tcp_keepalive_secs(mut self, secs: u64) -> Self {
        self.tcp_keepalive_secs = secs;
        self
    }

    pub fn build(self) -> Result<JsonPoller<T>, reqwest::Error> {
        let client = Client::builder()
            .pool_max_idle_per_host(self.pool_max_idle_per_host)
            .pool_idle_timeout(Duration::from_secs(self.pool_idle_timeout_secs))
            .timeout(Duration::from_millis(self.request_timeout_ms))
            .tcp_keepalive(Duration::from_secs(self.tcp_keepalive_secs))
            .build()?;

        Ok(JsonPoller {
            client,
            url: self.url,
            poll_interval: Duration::from_millis(self.poll_interval_ms),
            _phantom: PhantomData,
        })
    }
}

impl<T> JsonPoller<T>
where
    T: DeserializeOwned + Send,
{
    pub fn builder(url: impl Into<String>) -> JsonPollerBuilder<T> {
        JsonPollerBuilder::new(url)
    }

    pub async fn start<F, Fut>(&self, mut on_data: F)
    where
        F: FnMut(T, Duration) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        let mut interval_timer = interval(self.poll_interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval_timer.tick().await;
            let request_start = Instant::now();
            match self.fetch().await {
                Ok(data) => {
                    let elapsed = request_start.elapsed();
                    on_data(data, elapsed).await;
                }
                Err(e) => {
                    tracing::error!("Failed to fetch data: {:?}", e);
                }
            }
        }
    }

    async fn fetch(&self) -> Result<T, Box<dyn Error + Send + Sync>> {
        let response = self.client.get(&self.url).send().await.map_err(|e| {
            tracing::error!("Request failed: {:?}", e);
            Box::new(e) as Box<dyn Error + Send + Sync>
        })?;

        let status = response.status();
        if !status.is_success() {
            tracing::error!("HTTP error: {}", status);
            return Err(format!("HTTP {}", status).into());
        }

        let data = response.json::<T>().await.map_err(|e| {
            tracing::error!("JSON parse failed: {:?}", e);
            Box::new(e) as Box<dyn Error + Send + Sync>
        })?;

        Ok(data)
    }

    pub async fn fetch_once(&self) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        self.fetch().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct HttpBinJson {
        slideshow: Slideshow,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Slideshow {
        author: String,
        date: String,
        title: String,
        slides: Vec<Slide>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Slide {
        title: String,
        #[serde(rename = "type")]
        slide_type: String,
        #[serde(default)]
        items: Vec<String>,
    }

    #[test]
    fn test_builder_defaults() {
        let poller = JsonPoller::<HttpBinJson>::builder("https://example.com")
            .build()
            .unwrap();

        assert_eq!(
            poller.poll_interval,
            Duration::from_millis(POLL_INTERVAL_MS)
        );
        assert_eq!(poller.url, "https://example.com");
    }

    #[test]
    fn test_builder_custom_config() {
        let poller = JsonPoller::<HttpBinJson>::builder("https://example.com")
            .poll_interval_ms(1000)
            .request_timeout_ms(2000)
            .build()
            .unwrap();

        assert_eq!(poller.poll_interval, Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn test_http_error() {
        let poller = JsonPoller::<HttpBinJson>::builder("https://httpbin.org/status/404")
            .build()
            .unwrap();

        let result = poller.fetch_once().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let poller = JsonPoller::<HttpBinJson>::builder("https://httpbin.org/html")
            .build()
            .unwrap();

        let result = poller.fetch_once().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_once() {
        let json_poller = JsonPoller::<HttpBinJson>::builder("https://httpbin.org/json")
            .build()
            .unwrap();
        let data = json_poller.fetch_once().await.unwrap();

        assert_eq!(data.slideshow.author, "Yours Truly");
        assert_eq!(data.slideshow.title, "Sample Slide Show");
    }
}
