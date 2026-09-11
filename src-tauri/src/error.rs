use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CleanError {
    #[error("文件读写操作失败：{0}")]
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
    #[error("源文件在清理期间发生变化，已停止替换：{0}")]
    SourceChanged(String),
    #[error("源文件为只读，不能替换；请使用安全副本模式或先解除只读属性：{0}")]
    ReadOnly(String),
    #[error("ZIP 处理失败：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("PDF 处理失败：{0}")]
    Pdf(#[from] lopdf::Error),
}

pub type Result<T> = std::result::Result<T, CleanError>;

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::CleanError;

    #[test]
    fn io_errors_do_not_mislabel_write_failures_as_reads() {
        let error = CleanError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "access denied",
        ));
        assert_eq!(error.to_string(), "文件读写操作失败：access denied");
    }
}
