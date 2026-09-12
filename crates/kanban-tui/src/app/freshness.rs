use super::App;

impl App {
    #[doc(hidden)]
    pub async fn rewire_freshness(&mut self) -> Option<std::path::PathBuf> {
        None
    }
}
