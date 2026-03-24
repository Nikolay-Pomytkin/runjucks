//! Resolving template names to source strings for [`crate::Environment::render_template`].

use crate::errors::{Result, RunjucksError};
use std::collections::HashMap;
use std::sync::Arc;

/// Loads template source by name (e.g. `"layout.html"`).
///
/// Implement for in-memory maps, filesystem reads, or embedders that fetch from a CDN.
pub trait TemplateLoader: Send + Sync {
    fn load(&self, name: &str) -> Result<String>;
}

impl TemplateLoader for HashMap<String, String> {
    fn load(&self, name: &str) -> Result<String> {
        self.get(name)
            .cloned()
            .ok_or_else(|| RunjucksError::new(format!("template not found: {name}")))
    }
}

/// Wraps a closure as a [`TemplateLoader`].
pub struct FnLoader<F>(pub F);

impl<F> TemplateLoader for FnLoader<F>
where
    F: Fn(&str) -> Result<String> + Send + Sync,
{
    fn load(&self, name: &str) -> Result<String> {
        (self.0)(name)
    }
}

/// Helper to build an `Arc<dyn TemplateLoader>` from a map.
pub fn map_loader(map: HashMap<String, String>) -> Arc<dyn TemplateLoader + Send + Sync> {
    Arc::new(map)
}
