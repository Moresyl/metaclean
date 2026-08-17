use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CleanError {
    #[error("无法读取文件：{0}")]
    Io(#[from] std::io::Error),
    #[error("文件格式无效：{0}")]
    InvalidFormat(String),
    #[error("暂不支持此文件格式：{0}")]
    Unsupported(String),
    #[error("清理结果验证失败：{0}")]
    Verification(String),
    #[error("文件过大，最大允许 256 MiB：{0}")]
    TooLarge(String),
    #[error("拒绝处理符号链接：{0}")]
    Symlink(String),
    #[error("ZIP 处理失败：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("PDF 处理失败：{0}")]
    Pdf(#[from] lopdf::Error),
}

pub type Result<T> = std::result::Result<T, CleanError>;

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
