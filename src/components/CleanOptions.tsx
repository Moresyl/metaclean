import { Copy, ScanSearch } from "lucide-react";
import type { CleanMode } from "../types";
import { useI18n } from "../lib/i18n";

interface CleanOptionsProps { mode: CleanMode; onModeChange: (mode: CleanMode) => void; preserveTimestamps: boolean; onPreserveTimestampsChange: (value: boolean) => void; preserveOrientation: boolean; onPreserveOrientationChange: (value: boolean) => void; preserveColorProfile: boolean; onPreserveColorProfileChange: (value: boolean) => void; disabled: boolean; scanned: boolean; hasFindings: boolean; busy: boolean; onAction: () => void }

export default function CleanOptions({ mode, onModeChange, preserveTimestamps, onPreserveTimestampsChange, preserveOrientation, onPreserveOrientationChange, preserveColorProfile, onPreserveColorProfileChange, disabled, scanned, hasFindings, busy, onAction }: CleanOptionsProps) {
  const { text } = useI18n();
  return (
    <aside className="options-card">
      <div className="eyebrow">{text("清理方式", "Output mode")}</div>
      <button className={`mode-option ${mode === "copy" ? "selected" : ""}`} type="button" onClick={() => onModeChange("copy")}><Copy size={17} /><span><strong>{text("保存为安全副本", "Save a safe copy")}</strong><small>{text("推荐，不修改原文件", "Recommended; keeps the original")}</small></span></button>
      <button className={`mode-option ${mode === "replace" ? "selected" : ""}`} type="button" onClick={() => onModeChange("replace")}><ScanSearch size={17} /><span><strong>{text("替换原文件", "Replace original")}</strong><small>{text("处理前自动创建备份", "Creates a backup before cleaning")}</small></span></button>
      <div className="privacy-options"><label><input type="checkbox" checked={preserveOrientation} onChange={(event) => onPreserveOrientationChange(event.target.checked)}/><span>{text("保留照片方向", "Preserve photo orientation")}</span></label><label><input type="checkbox" checked={preserveColorProfile} onChange={(event) => onPreserveColorProfileChange(event.target.checked)}/><span>{text("图片", "Images")} · ICC / sRGB</span></label><label><input type="checkbox" checked={preserveTimestamps} onChange={(event) => onPreserveTimestampsChange(event.target.checked)}/><span>{text("保留文件时间戳", "Preserve file timestamps")}</span></label></div>
      <div className="option-divider" />
      <div className="scan-summary"><span>{text("将检测", "Scans for")}</span><ul><li>EXIF / GPS</li><li>{text("音频标签与封面", "Audio tags and artwork")}</li><li>{text("视频用户数据与位置", "Video user data and location")}</li><li>{text("文档作者与修订", "Document authors and revisions")}</li><li>PDF {text("属性与 XMP", "properties and XMP")}</li><li>{text("不可见 Unicode", "Invisible Unicode")}</li></ul></div>
      <button className="scan-button" type="button" disabled={disabled || busy || (scanned && !hasFindings)} onClick={onAction}><ScanSearch size={17} />{busy ? text("处理中…", "Working…") : scanned ? hasFindings ? text("确认并开始清理", "Confirm and clean") : text("没有需要清理的痕迹", "No traces to clean") : text("扫描隐私痕迹", "Scan privacy traces")}</button>
      <p className="safety-copy">{scanned ? text("只清理报告中列出的痕迹。", "Only listed traces will be removed.") : text("扫描是只读操作，不会修改文件。", "Scanning is read-only and changes nothing.")}</p>
    </aside>
  );
}
