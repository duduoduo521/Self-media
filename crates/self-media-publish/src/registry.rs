use std::collections::HashMap;

use self_media_core::types::Platform;

use crate::publisher::PlatformPublisher;

/// 平台适配器注册表
pub struct PublisherRegistry {
    publishers: HashMap<Platform, Box<dyn PlatformPublisher>>,
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
    pub fn register(&mut self, publisher: Box<dyn PlatformPublisher>) {
        let platform = publisher.platform();
        self.publishers.insert(platform, publisher);
    }

    /// 获取平台适配器
    pub fn get(&self, platform: &Platform) -> Option<&dyn PlatformPublisher> {
        self.publishers.get(platform).map(|p| p.as_ref())
    }

    /// 获取所有已注册平台
    pub fn all_platforms(&self) -> Vec<Platform> {
        self.publishers.keys().cloned().collect()
    }
}
