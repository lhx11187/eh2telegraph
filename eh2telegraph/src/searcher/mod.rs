pub mod f_hash;
pub mod saucenao;

pub trait ImageSearcher {
    type SeacheError;
    type SearchOutput;
    type FetchFuture<T>: std::future::Future<Output = Result<Self::SearchOutput, Self::SeacheError>>;

    fn search<T: Into<std::borrow::Cow<'static, [u8]>>>(&self, data: T) -> Self::FetchFuture<T>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore]
    #[tokio::test]
    async fn demo() {
        let data = std::fs::read("./image.png").unwrap();
        let searcher = saucenao::SaucenaoSearcher::new(None);
        let r = searcher.search(data).await;
        println!("result: {r:?}");
    }
}
