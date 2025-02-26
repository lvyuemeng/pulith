use crate::utils::{
    task_pool::POOL,
    ui::tracker::{ProgressTrackerBuilder, Tracker, TrackerBuilder},
};
use anyhow::{Result, bail};
use reqwest::{Client, Proxy, Response, Url};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncWriteExt};

struct FileDownload<U: Into<Url>> {
    url: U,
    path_name: PathBuf,
}

impl<U: Into<Url>> FileDownload<U> {
    pub fn new(url: U, path_name: &Path) -> Self {
        Self {
            url,
            path_name: path_name.to_path_buf(),
        }
    }

    // TODO!: Add other kinds of tracker
    pub fn fetch_raw(self, tb: Option<ProgressTrackerBuilder>) -> Result<()> {
        POOL.block_on(async move {
            let mut res = Download::fetch(self.url).await?;
            let mut file = File::create(&self.path_name).await?;

            let mut t = tb
                .zip(res.content_length())
                .map(|(tb_, len)| tb_.with_len(len).build());

            while let Some(chunk) = res.chunk().await? {
                file.write_all(&chunk).await?;
                t.as_mut().map(|t| t.step(chunk.len() as u64));
            }

            t.map(|t| t.finish());
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
