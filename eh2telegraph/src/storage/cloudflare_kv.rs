use std::{sync::Arc, time::Duration};

use cloudflare_kv_proxy::{Client, ClientError, NotFoundMapping};
use futures::Future;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::config;

use super::KVStorage;

const CONFIG_KEY: &str = "worker_kv";

#[derive(Debug, Deserialize)]
pub struct CFConfig {
    pub endpoint: String,
    pub token: String,
    pub cache_size: usize,
    pub expire_sec: u64,
}

#[derive(Clone, Debug)]
pub struct CFStorage(Arc<Client>);

impl CFStorage {
    pub fn new<T: Into<String>, E: Into<String>>(
        endpoint: E,
        token: T,
        cache_size: usize,
        expire: Duration,
    ) -> Result<Self, ClientError> {
        Ok(Self(Arc::new(Client::new(
            endpoint, token, cache_size, expire,
        )?)))
    }

    pub fn new_from_config() -> anyhow::Result<Self> {
        let config: CFConfig = config::parse(CONFIG_KEY)?
            .ok_or_else(|| anyhow::anyhow!("cloudflare worker config(key: worker_kv) not found"))?;
        Self::new(
            config.endpoint,
            config.token,
            config.cache_size,
            Duration::from_secs(config.expire_sec),
        )
        .map_err(Into::into)
    }
}

impl<T> KVStorage<T> for CFStorage
where
    T: DeserializeOwned + Serialize + Send + Sync,
{
    type GetFuture<'a> = impl Future<Output = anyhow::Result<Option<T>>> where Self: 'a;
    fn get<'a>(&'a self, key: &'a str) -> Self::GetFuture<'_> {
        async move {
            self.0
                .get(key)
                .await
                .map_not_found_to_option()
                .map_err(Into::into)
        }
    }

    type SetFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn set<'a>(&self, key: String, value: T, _expire_ttl: Option<usize>) -> Self::SetFuture<'_> {
        async move { self.0.put(&key, &value).await.map_err(Into::into) }
    }

    type DeleteFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn delete<'a>(&'a self, key: &'a str) -> Self::DeleteFuture<'_> {
        async move { self.0.delete(key).await.map_err(Into::into) }
    }
}
