use crate::utils::{
        task_pool::POOL,
        ui::tracker::{ProgressTracker, ProgressTrackerConfig, Tracker},
    };
use anyhow::{Result, bail};
use reqwest::{Client, Proxy, Response, Url};
use std::path::Path;
use tokio::{fs::File, io::AsyncWriteExt};

struct FileDownload;

impl FileDownload {
    pub fn fetch_raw(url: impl Into<Url>, path_name: impl AsRef<Path>) -> Result<()> {
        POOL.block_on(async move {
            let mut res = Download::fetch(url).await?;
            let mut file = File::create(&path_name).await?;

            let len = res.content_length();
            let config = ProgressTrackerConfig { len, msg: None };
            let t = ProgressTracker::new(config);

            while let Some(chunk) = res.chunk().await? {
                file.write_all(&chunk).await?;
                t.step(chunk.len() as u64);
            }
            t.finish(Some("Download completed".to_string()));
            Ok(())
        })
    }
}

struct Download;

impl Download {
    pub async fn fetch(url: impl Into<Url>) -> Result<Response> {
        // get setting from env
        let cs = ClientSetting::default();
        let Ok(c) = cs.build() else {
            bail!("Failed to build client")
        };

        c.get(url.into())
            .send()
            .await
            .map_err(|e| anyhow::Error::from(e))
    }
}

#[derive(Clone, Default)]
pub struct ClientSetting {
    pub proxies: Option<Vec<Url>>,
}

impl ClientSetting {
    pub fn build(self) -> Result<Client> {
        let mut cb = Client::builder();

        if let Some(proxies) = self.proxies {
            let (secure, insecure): (Vec<Url>, Vec<Url>) =
                proxies.into_iter().partition(|u| u.scheme() == "https");

            for u in secure {
                cb = cb.proxy(Proxy::https(u)?);
            }

            for u in insecure {
                cb = cb.proxy(Proxy::http(u)?);
            }
        }

        Ok(cb.build()?)
    }
}
