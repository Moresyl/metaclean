import { Copy, FileWarning, RotateCcw } from "lucide-react";
import type { CleanMode } from "../types";

export default function SettingsPage({ mode, onModeChange }: { mode: CleanMode; onModeChange: (mode: CleanMode) => void }) {
  return <section className="secondary-page"><div className="page-heading"><div><h1>设置</h1><p>设置会自动保存在本机。</p></div></div><div className="settings-card"><h2>默认输出方式</h2><p>每次添加新文件时使用的默认策略。</p><div className="settings-choices"><button className={mode === "copy" ? "selected" : ""} type="button" onClick={() => onModeChange("copy")}><Copy size={18}/><span><strong>保存为安全副本</strong><small>保留原文件，生成 .cleaned 副本</small></span></button><button className={mode === "replace" ? "selected" : ""} type="button" onClick={() => onModeChange("replace")}><FileWarning size={18}/><span><strong>替换并备份</strong><small>先创建 .bak，再原子替换原文件</small></span></button></div></div><div className="settings-card"><h2>安全保证</h2><div className="safety-grid"><span><RotateCcw size={16}/>替换模式始终备份</span><span>单文件最大 256 MiB</span><span>拒绝写入符号链接</span><span>输出采用临时文件原子替换</span></div></div></section>;
}
