pub mod weibo;
pub mod toutiao;
pub mod wechat;
pub mod bilibili;
pub mod xiaohongshu;
pub mod douyin;

use reqwest::Client;

use crate::registry::PublisherRegistry;

/// 注册所有平台适配器
pub fn register_all(registry: &mut PublisherRegistry) {
    let http = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("HTTP client build failed");

    registry.register(Box::new(weibo::WeiboPublisher::new(http.clone())));
    registry.register(Box::new(toutiao::ToutiaoPublisher::new(http.clone())));
    registry.register(Box::new(wechat::WeChatPublisher::new(http.clone())));
    registry.register(Box::new(bilibili::BilibiliPublisher::new(http.clone())));
    registry.register(Box::new(xiaohongshu::XiaohongshuPublisher::new(http.clone())));
    registry.register(Box::new(douyin::DouyinPublisher::new(http)));
}
