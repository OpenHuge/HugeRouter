use plugin_sdk::PluginManifest;

#[derive(Debug, Default)]
pub struct PluginRegistry {
    manifests: Vec<PluginManifest>,
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, manifest: PluginManifest) {
        self.manifests.push(manifest);
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.manifests.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}
