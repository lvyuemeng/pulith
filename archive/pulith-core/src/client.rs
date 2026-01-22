use crate::{
    task_pool::POOL,
    ui::tracker::{ProgressTrackerBuilder, Tracker, TrackerBuilder},
};

use reqwest::{Client, Proxy, Response, Url};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt};

pub struct FileDownload<U: Into<Url>> {
    url: U,
    path_name: PathBuf,
}

#[derive(Error, Debug)]
pub enum FileDownloadError {
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error(transparent)]
    Io(#[from] tokio::io::Error),
    #[error(transparent)]
    Req(#[from] reqwest::Error),
}

impl<U: Into<Url>> FileDownload<U> {
    pub fn new(url: U, path_name: &Path) -> Self {
        Self {
            url,
            path_name: path_name.to_path_buf(),
        }
    }

    // TODO!: Add other kinds of tracker
    pub fn fetch_raw(self, tb: Option<ProgressTrackerBuilder>) -> Result<(), FileDownloadError> {
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

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Failed to build client: {source}")]
    ClientBuild {
        #[from]
        source: ClientSettingError,
    },
    #[error(transparent)]
    Req(#[from] reqwest::Error),
}

impl Download {
    pub async fn fetch(url: impl Into<Url>) -> Result<Response, DownloadError> {
        // get setting from env
        let cs = ClientSetting::default();
        let c = cs
            .build()
            .map_err(|source| DownloadError::ClientBuild { source })?;

        c.get(url.into()).send().await.map_err(DownloadError::Req)
    }
}

#[derive(Debug, Error)]
pub enum ClientSettingError {
    #[error("Invalid proxy URL {url}: {source}")]
    Proxy {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("Failed to build client: {0}")]
    Build(#[from] reqwest::Error),
}

#[derive(Clone, Default)]
pub struct ClientSetting {
    pub proxies: Option<Vec<Url>>,
}

impl ClientSetting {
    pub fn build(self) -> Result<Client, ClientSettingError> {
        let mut cb = Client::builder();

        if let Some(proxies) = self.proxies {
            let (secure, insecure): (Vec<Url>, Vec<Url>) =
                proxies.into_iter().partition(|u| u.scheme() == "https");

            for u in secure {
                cb = cb.proxy(Proxy::https(&u.to_string()).map_err(|source| {
                    ClientSettingError::Proxy {
                        url: u.to_string(),
                        source: source,
                    }
                })?);
            }

            for u in insecure {
                cb = cb.proxy(Proxy::http(&u.to_string()).map_err(|source| {
                    ClientSettingError::Proxy {
                        url: u.to_string(),
                        source,
                    }
                })?);
            }
        }

        Ok(cb.build().map_err(ClientSettingError::Build)?)
    }
}
