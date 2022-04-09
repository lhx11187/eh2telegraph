// A wrapper for reqwest to provide ability to bind to random ip.
// Since apparently I can not afford a ipv4 subnet, here I assume ipv6.
// Using he.net tunnel broker works fine.
// Setup:
// 1. sudo ip add add local 2001:x:x::/48 dev lo
// 2. sudo ip route add local 2001:x:x::/48 dev he-ipv6
// 3. Set net.ipv6.ip_nonlocal_bind=1

pub const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.99 Safari/537.36";
const CONFIG_KEY: &str = "http";

use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use ipnet::Ipv6Net;
use reqwest::header;
use rustls::ClientConfig;

use crate::{config, tls::WhitelistVerifier};

const CF_ADDR: Ipv6Addr = Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111);
const TG_ADDR: Ipv6Addr = Ipv6Addr::new(0x2001, 0x67c, 0x4e8, 0x1033, 0x1, 0x100, 0, 0xa);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::Into)]
pub struct Ipv6Net2(Ipv6Net);

impl<'de> serde::Deserialize<'de> for Ipv6Net2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::str::FromStr;
        let data = String::deserialize(deserializer)?;
        Ipv6Net::from_str(&data)
            .map(Ipv6Net2)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(serde::Deserialize, Clone, Debug, Default)]
struct HTTPConfig {
    ipv6_prefix: Option<Ipv6Net2>,
}

#[derive(Debug, Default)]
pub struct GhostClientBuilder {
    mapping: Vec<(&'static str, SocketAddr)>,
    headers: Option<header::HeaderMap>,
}

impl GhostClientBuilder {
    pub fn with_default_headers(self, headers: header::HeaderMap) -> Self {
        Self {
            headers: Some(headers),
            ..self
        }
    }

    pub fn with_cf_resolve(mut self, domains: &[&'static str]) -> Self {
        let cf = SocketAddr::new(IpAddr::V6(CF_ADDR), 443);
        for &domain in domains.iter() {
            self.mapping.push((domain, cf));
        }
        self
    }

    #[deprecated = "telegra.ph has fixed it and returns 501 when using ipv6"]
    pub fn with_tg_resolve(mut self) -> Self {
        let tg = SocketAddr::new(IpAddr::V6(TG_ADDR), 443);
        self.mapping.push(("telegra.ph", tg));
        self.mapping.push(("api.telegra.ph", tg));
        self
    }

    pub fn build(self, prefix: Option<Ipv6Net>) -> GhostClient {
        let inner = GhostClient::build_raw(&prefix, &self.mapping, self.headers.clone());
        GhostClient {
            prefix,
            mapping: Arc::new(self.mapping),
            headers: self.headers,
            inner,
        }
    }

    pub fn build_from_config(self) -> anyhow::Result<GhostClient> {
        let config: HTTPConfig = config::parse(CONFIG_KEY)?.unwrap_or_default();
        let prefix = config.ipv6_prefix.map(Into::into);
        Ok(self.build(prefix))
    }
}

#[derive(Debug, Default)]
pub struct GhostClient {
    prefix: Option<Ipv6Net>,
    mapping: Arc<Vec<(&'static str, SocketAddr)>>,
    headers: Option<header::HeaderMap>,

    inner: reqwest::Client,
}

impl GhostClient {
    pub fn builder() -> GhostClientBuilder {
        GhostClientBuilder::default()
    }
}

impl Clone for GhostClient {
    fn clone(&self) -> Self {
        let inner = Self::build_raw(&self.prefix, &self.mapping, self.headers.clone());
        Self {
            prefix: self.prefix,
            mapping: self.mapping.clone(),
            headers: self.headers.clone(),
            inner,
        }
    }
}

impl Deref for GhostClient {
    type Target = reqwest::Client;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for GhostClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl GhostClient {
    fn build_raw(
        net: &Option<Ipv6Net>,
        mapping: &[(&'static str, SocketAddr)],
        headers: Option<header::HeaderMap>,
    ) -> reqwest::Client {
        let mut builder = reqwest::Client::builder().user_agent(UA);

        if let Some(headers) = headers {
            builder = builder.default_headers(headers);
        }

        if let Some(net) = net {
            let addr: u128 = net.addr().into();
            let prefix_len = net.prefix_len();
            let mask = !u128::max_value()
                .checked_shl((128 - prefix_len) as u32)
                .unwrap_or(u128::min_value());

            // use random ipv6
            let rand: u128 = rand::Rng::gen(&mut rand::thread_rng());
            let addr = IpAddr::V6(Ipv6Addr::from(rand & mask | addr));
            builder = builder.local_address(addr);

            // apply resolve
            for (domain, addr) in mapping {
                builder = builder.resolve(*domain, *addr);
            }

            // not add preconfigured tls
            // let tls_config = TLS_CFG.clone();
            // builder = builder.use_preconfigured_tls(tls_config);
        }

        builder.build().expect("build reqwest client failed")
    }

    pub fn refresh(&mut self) {
        self.inner = Self::build_raw(&self.prefix, &self.mapping, self.headers.clone());
    }
}

lazy_static::lazy_static! {
    // here we only meet telegra.ph with wrong tls config, so we write them as fixed values.
    static ref TLS_CFG: ClientConfig = WhitelistVerifier::new(["telegram.org"]).into();
}

#[cfg(test)]
mod tests {
    use super::{TLS_CFG, UA};

    #[ignore]
    #[tokio::test]
    async fn test_tls() {
        let tls_config = TLS_CFG.clone();
        // use a telegram.org ip address(normally it fails in browser)
        let cli = reqwest::Client::builder()
            .user_agent(UA)
            .resolve("api.telegra.ph", "149.154.167.99:443".parse().unwrap())
            .use_preconfigured_tls(tls_config)
            .build()
            .unwrap();
        let resp = cli
            .get("https://api.telegra.ph/getPage")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }
}
