use std::sync::Arc;

use futures::Future;
use hashlink::LruCache;
use parking_lot::Mutex;

use super::KVStorage;

#[derive(Clone, Debug)]
pub struct LruStorage(Arc<Mutex<LruCache<String, String>>>);

impl LruStorage {
    pub fn new(capacity: usize) -> Self {
        Self(Arc::new(Mutex::new(LruCache::new(capacity))))
    }
}

impl KVStorage<String> for LruStorage {
    type GetFuture<'a> = impl Future<Output = anyhow::Result<Option<String>>> where Self: 'a;
    fn get<'a>(&'a self, key: &'a str) -> Self::GetFuture<'_> {
        let v = self.0.lock().get(key).cloned();
        async move { Ok(v) }
    }

    type SetFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn set<'a>(
        &self,
        key: String,
        value: String,
        _expire_ttl: Option<usize>,
    ) -> Self::SetFuture<'_> {
        self.0.lock().insert(key, value);
        async move { Ok(()) }
    }

    type DeleteFuture<'a> = impl Future<Output = anyhow::Result<()>> where Self: 'a;
    fn delete<'a>(&'a self, key: &'a str) -> Self::DeleteFuture<'_> {
        self.0.lock().remove(key);
        async move { Ok(()) }
    }
}
