use crate::{PersistenceError, PersistenceStore};
use std::sync::Arc;

pub trait StoreFactory: Send + Sync {
    fn name(&self) -> &str;
    fn supported_patterns(&self) -> &[&str];
    fn matches(&self, locator: &str) -> bool;
    fn create(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError>;
}

pub struct StoreRegistry {
    factories: Vec<Box<dyn StoreFactory>>,
}

impl StoreRegistry {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    pub fn register(&mut self, factory: Box<dyn StoreFactory>) {
        self.factories.push(factory);
    }

    pub fn create_store(
        &self,
        locator: &str,
    ) -> Result<Arc<dyn PersistenceStore + Send + Sync>, PersistenceError> {
        for factory in &self.factories {
            if factory.matches(locator) {
                return factory.create(locator);
            }
        }
        let supported: Vec<String> = self
            .factories
            .iter()
            .flat_map(|f| f.supported_patterns().iter().map(|s| (*s).to_string()))
            .collect();
        Err(PersistenceError::UnsupportedLocator {
            locator: locator.to_string(),
            supported,
        })
    }
}

impl Default for StoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}
