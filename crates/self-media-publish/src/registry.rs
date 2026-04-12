use std::collections::HashMap;
use std::sync::Arc;

use self_media_core::types::Platform;

use crate::publisher::PlatformPublisher;

/// 平台适配器注册表
pub struct PublisherRegistry {
    publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
}

impl Default for PublisherRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PublisherRegistry {
    pub fn new() -> Self {
        Self {
            publishers: HashMap::new(),
        }
    }

    /// 注册平台适配器
    pub fn register(&mut self, publisher: Arc<dyn PlatformPublisher>) {
        let platform = publisher.platform();
        self.publishers.insert(platform, publisher);
    }

    /// 获取平台适配器
    pub fn get(&self, platform: &Platform) -> Option<Arc<dyn PlatformPublisher>> {
        self.publishers.get(platform).cloned()
    }

    /// 获取所有已注册平台
    pub fn all_platforms(&self) -> Vec<Platform> {
        self.publishers.keys().cloned().collect()
    }
}
