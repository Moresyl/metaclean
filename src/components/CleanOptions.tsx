import { Copy, ScanSearch } from "lucide-react";
import type { CleanMode } from "../types";

interface CleanOptionsProps { mode: CleanMode; onModeChange: (mode: CleanMode) => void; disabled: boolean }

export default function CleanOptions({ mode, onModeChange, disabled }: CleanOptionsProps) {
  return (
    <aside className="options-card">
      <div className="eyebrow">清理方式</div>
      <button className={`mode-option ${mode === "copy" ? "selected" : ""}`} type="button" onClick={() => onModeChange("copy")}><Copy size={17} /><span><strong>保存为安全副本</strong><small>推荐，不修改原文件</small></span></button>
      <button className={`mode-option ${mode === "replace" ? "selected" : ""}`} type="button" onClick={() => onModeChange("replace")}><ScanSearch size={17} /><span><strong>替换原文件</strong><small>处理前自动创建备份</small></span></button>
      <div className="option-divider" />
      <div className="scan-summary"><span>将检测</span><ul><li>EXIF / GPS 位置</li><li>文档作者与修订</li><li>PDF 属性与 XMP</li><li>不可见 Unicode</li></ul></div>
      <button className="scan-button" type="button" disabled={disabled}><ScanSearch size={17} />扫描隐私痕迹</button>
      <p className="safety-copy">扫描是只读操作，不会修改文件。</p>
    </aside>
  );
}
