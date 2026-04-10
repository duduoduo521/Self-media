pub mod weibo;
pub mod toutiao;
pub mod wechat;
pub mod bilibili;
pub mod xiaohongshu;
pub mod douyin;

// 扫码登录模块
pub mod weibo_qr;
pub mod bilibili_qr;
pub mod toutiao_qr;
pub mod douyin_qr;
pub mod xiaohongshu_qr;
pub mod wechat_qr;

use reqwest::Client;

use crate::registry::PublisherRegistry;
use crate::qr_login::QrLoginManager;

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

/// 注册扫码登录处理器
pub fn register_qr_handlers(manager: &mut QrLoginManager) {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("HTTP client build failed");

    // 微博/B站扫码登录（完整实现）
    manager.register_handler(weibo_qr::WeiboQrLogin::new(http.clone()));
    manager.register_handler(bilibili_qr::BilibiliQrLogin::new(http.clone()));
    
    // 头条/抖音/小红书/公众号（OAuth 框架）
    manager.register_handler(toutiao_qr::ToutiaoQrLogin::new(http.clone()));
    manager.register_handler(douyin_qr::DouyinQrLogin::new(http.clone()));
    manager.register_handler(xiaohongshu_qr::XiaohongshuQrLogin::new(http.clone()));
    manager.register_handler(wechat_qr::WechatQrLogin::new(http));
}
