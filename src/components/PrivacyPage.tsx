import { FileLock2, HardDrive, ShieldCheck } from "lucide-react";

export default function PrivacyPage() {
  return <section className="secondary-page"><div className="page-heading"><div><h1>隐私说明</h1><p>MetaClean 的处理边界清晰且可验证。</p></div></div><div className="privacy-grid"><article><HardDrive size={21}/><h2>纯本地运行</h2><p>扫描和清理均由本机 Rust 引擎完成，没有上传接口或云端处理。</p></article><article><ShieldCheck size={21}/><h2>先扫描后清理</h2><p>扫描阶段只读。只有确认后才生成安全副本或替换原文件。</p></article><article><FileLock2 size={21}/><h2>处理你有权处理的内容</h2><p>工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或处理未获授权的内容。</p></article></div><div className="scope-card"><h2>当前支持范围</h2><p>JPEG、PNG、WebP 图片元数据；DOCX、XLSX、PPTX、ODT 文档属性及批注；PDF 属性和 XMP；TXT、Markdown、HTML、SVG 等文本中的不可见字符与生成器元数据。</p><p>统计型文本水印、像素域水印、视频和传统二进制 Office 格式不在处理范围内。</p></div></section>;
}
