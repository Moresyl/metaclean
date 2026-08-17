# MetaClean v0.2.0

## 中文

### 新功能

- 新增稳定版本发现：启动后非阻塞检查、设置页手动检查、全局更新提示和官方发布页跳转。自动检查可以关闭。
- 新增文件夹选择与递归拖入，支持稳定排序、去重、跳过计数和首个失败原因；单次最多 10,000 个文件、最多 64 层目录。
- 新增 GIF、MP3、WAV、FLAC 清理：覆盖 GIF 注释/XMP、ID3v1/ID3v2/APEv2、RIFF INFO/XMP/BWF/iXML，以及 FLAC Vorbis 评论、封面和 XMP。
- 新增 JPEG 方向保留和文件时间戳保留开关。JPEG 只重建一个最小 Orientation 字段，不保留 GPS、作者、设备或拍摄参数。

### 变更与安全

- Windows 资源管理器右键菜单从 18 种扩展名扩展到 22 种。
- 递归导入拒绝符号链接并限制深度与数量，防止目录环和资源耗尽。
- 更新检查拒绝草稿、预发布和非官方仓库链接；文件内容永远不会进入更新请求。
- 新媒体解析器在写入前验证块边界、长度与必要结构，畸形容器直接失败且不修改源文件。

### 修复与打磨

- 清理后的文件默认保留原权限与访问/修改时间。
- 修复删除全部 EXIF 后部分 JPEG 在查看器中旋转的问题。
- 测试范围固定在产品源码目录，避免把文档中的临时示例工程误收集为产品测试。

### 安装

- Windows x64：优先使用 `MetaClean_0.2.0_x64-setup.exe`，也提供 MSI。
- macOS：Apple Silicon 使用 `MetaClean_0.2.0_aarch64.dmg`，Intel 使用 `MetaClean_0.2.0_x64.dmg`。
- Linux x64：提供 AppImage、DEB 和 RPM。

## English summary

MetaClean 0.2.0 adds stable-release discovery, recursive folder intake, GIF/MP3/WAV/FLAC metadata cleaning, minimal JPEG orientation preservation, and filesystem timestamp/permission preservation. The new parsers fail closed on malformed containers, recursive intake refuses symlinks and enforces resource limits, and update links are restricted to the official repository. Windows Explorer integration now covers 22 extensions.
