use futures::Future;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

pub mod cloudflare_kv;
pub mod lru;

pub trait KVStorage<V> {
    type GetFuture<'a>: Future<Output = anyhow::Result<Option<V>>> + Send
    where
        Self: 'a;
    fn get<'a>(&'a self, key: &'a str) -> Self::GetFuture<'_>;

    type SetFuture<'a>: Future<Output = anyhow::Result<()>> + Send
    where
        Self: 'a;
    fn set(&self, key: String, value: V, expire_ttl: Option<usize>) -> Self::SetFuture<'_>;

    type DeleteFuture<'a>: Future<Output = anyhow::Result<()>> + Send
    where
        Self: 'a;
    fn delete<'a>(&'a self, key: &'a str) -> Self::DeleteFuture<'_>;
}

#[derive(Default, Clone, Debug)]
pub struct SimpleMemStorage(Arc<RwLock<HashMap<String, String>>>);

impl SimpleMemStorage {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Arc::new(RwLock::new(HashMap::with_capacity(capacity))))
    }
}

impl KVStorage<String> for SimpleMemStorage {
    type GetFuture<'a> = impl Future<Output = anyhow::Result<Option<String>>> where Self: 'a;
    fn get<'a>(&'a self, key: &'a str) -> Self::GetFuture<'_> {
        let v = self.0.read().get(key).cloned();
        async move { Ok(v) }
    }

    type SetFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn set<'a>(
        &self,
        key: String,
        value: String,
        _expire_ttl: Option<usize>,
    ) -> Self::SetFuture<'_> {
        self.0.write().insert(key, value);
        async move { Ok(()) }
    }

    type DeleteFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn delete<'a>(&'a self, key: &'a str) -> Self::DeleteFuture<'_> {
        self.0.write().remove(key);
        async move { Ok(()) }
    }
}
