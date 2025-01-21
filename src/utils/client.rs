use anyhow::{Result, bail};
use reqwest::{Client, Proxy, Response, Url};
struct Download;

impl Download {
    pub async fn fetch(url: Url) -> Result<Response> {
        let cs = ClientSetting::default();
        let Ok(c) = cs.build() else {
            bail!("Failed to build client")
        };

        c.get(url).send().await.map_err(|e| anyhow::Error::from(e))
    }
}

#[derive(Clone, Default)]
pub struct ClientSetting {
    pub proxies: Option<Vec<Url>>,
}

impl ClientSetting {
    pub fn build(self) -> Result<Client> {
        if let Some(proxies) = self.proxies {
            let (secure, insecure): (Vec<Url>, Vec<Url>) =
                proxies.into_iter().partition(|u| u.scheme() == "https");

            let mut cb = Client::builder();

            for u in secure {
                cb = cb.proxy(Proxy::https(u)?);
            }

            for u in insecure {
                cb = cb.proxy(Proxy::http(u)?);
            }

            return Ok(cb.build()?);
        }
        Ok(Client::new())
    }
}
