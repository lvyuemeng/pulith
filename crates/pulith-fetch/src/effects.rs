use bytes::Bytes;
use futures_util::Stream;
use futures_util::TryStreamExt;
use pulith_fs::workflow;
use pulith_verify::Hasher;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::io::AsyncWriteExt;

pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + Sync + 'a>>;

pub trait HttpClient: Send + Sync {
    type Error: std::error::Error + Send + 'static;

    fn stream(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<BoxStream<'static, Result<Bytes, Self::Error>>, Self::Error>> + Send;
    fn head(&self, url: &str) -> impl Future<Output = Result<Option<u64>, Self::Error>> + Send;
}

pub struct Fetcher<C: HttpClient> {
    client: C,
    workspace_root: PathBuf,
    options: crate::data::FetchOptions,
}

impl<C: HttpClient> Fetcher<C> {
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            client,
            workspace_root: workspace_root.into(),
            options: crate::data::FetchOptions::default(),
        }
    }

    pub fn with_options(mut self, options: crate::data::FetchOptions) -> Self {
        self.options = options;
        self
    }

    pub async fn fetch(
        &self,
        url: &str,
        destination: &Path,
    ) -> Result<PathBuf, crate::error::FetchError> {
        self.notify_progress(crate::data::Progress {
            phase: crate::data::FetchPhase::Connecting,
            bytes_downloaded: 0,
            total_bytes: None,
            retry_count: 0,
        });

        let ws = workflow::Workspace::new(&self.workspace_root, destination)
            .map_err(crate::error::FetchError::Fs)?;

        let staging_path = ws.path().join("download.tmp");
        let bytes_downloaded = self.stream_to_staging(url, &staging_path).await?;

        self.notify_progress(crate::data::Progress {
            phase: crate::data::FetchPhase::Verifying,
            bytes_downloaded,
            total_bytes: None,
            retry_count: 0,
        });

        ws.commit().map_err(crate::error::FetchError::Fs)?;

        self.notify_progress(crate::data::Progress {
            phase: crate::data::FetchPhase::Completed,
            bytes_downloaded,
            total_bytes: None,
            retry_count: 0,
        });

        Ok(destination.to_path_buf())
    }

    fn map_error<E: std::error::Error + Send + 'static>(e: E) -> crate::error::FetchError {
        crate::error::FetchError::Network(e.to_string())
    }

    async fn stream_to_staging(
        &self,
        url: &str,
        staging_path: &Path,
    ) -> Result<u64, crate::error::FetchError> {
        let mut stream = self.client.stream(url).await.map_err(Self::map_error)?;
        let mut file = tokio::fs::File::create(staging_path)
            .await
            .map_err(Self::map_error)?;
        let mut hasher: Option<pulith_verify::Sha256Hasher> = self
            .options
            .checksum
            .as_ref()
            .map(|_| pulith_verify::Sha256Hasher::new());

        let mut bytes_downloaded = 0u64;

        while let Some(chunk_result) = stream.try_next().await.transpose() {
            let bytes = chunk_result.map_err(Self::map_error)?;

            if let Some(ref mut h) = hasher {
                h.update(&bytes);
            }
            file.write_all(&bytes).await.map_err(Self::map_error)?;

            bytes_downloaded += bytes.len() as u64;

            if let Some(ref callback) = self.options.on_progress {
                callback(crate::data::Progress {
                    phase: crate::data::FetchPhase::Downloading,
                    bytes_downloaded,
                    total_bytes: None,
                    retry_count: 0,
                });
            }
        }

        file.sync_all().await.map_err(Self::map_error)?;

        if let Some(h) = hasher {
            let actual = h.finalize();
            if let Some(ref expected) = self.options.checksum
                && actual.as_slice() != expected.as_slice()
            {
                return Err(crate::error::FetchError::ChecksumMismatch {
                    expected: hex::encode(expected),
                    actual: hex::encode(actual),
                });
            }
        }

        Ok(bytes_downloaded)
    }

    fn notify_progress(&self, progress: crate::data::Progress) {
        if let Some(ref callback) = self.options.on_progress {
            callback(progress);
        }
    }
}

#[cfg(feature = "reqwest")]
mod reqwest_client {
    use super::*;
    use reqwest::Client;

    pub struct ReqwestClient {
        client: Client,
    }

    impl ReqwestClient {
        pub fn new() -> Result<Self, reqwest::Error> {
            let client = Client::builder().build()?;
            Ok(Self { client })
        }
    }

    impl HttpClient for ReqwestClient {
        type Error = reqwest::Error;

        async fn stream(
            &self,
            url: &str,
        ) -> Result<BoxStream<'static, Result<Bytes, Self::Error>>, Self::Error> {
            let response = self.client.get(url).send().await?;
            let stream = response.bytes_stream().map_ok(Bytes::from);
            Ok(Box::pin(stream))
        }

        async fn head(&self, url: &str) -> Result<Option<u64>, Self::Error> {
            self.client
                .head(url)
                .send()
                .await?
                .content_length()
                .map(Ok)
                .transpose()
        }
    }
}

#[cfg(feature = "reqwest")]
pub use reqwest_client::ReqwestClient;
