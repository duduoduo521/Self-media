#[cfg(test)]
mod tests {
    use crate::types::{Platform, TaskMode, TaskStatus};

    #[test]
    fn test_platform_count() {
        let platforms = vec![
            Platform::Weibo,
            Platform::Bilibili,
            Platform::Douyin,
            Platform::Xiaohongshu,
            Platform::Toutiao,
            Platform::WeChatOfficial,
        ];
        assert_eq!(platforms.len(), 6);
    }

    #[test]
    fn test_task_status_count() {
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];
        assert_eq!(statuses.len(), 5);
    }

    #[test]
    fn test_task_mode_count() {
        let modes = vec![TaskMode::Text, TaskMode::Video];
        assert_eq!(modes.len(), 2);
    }
}
