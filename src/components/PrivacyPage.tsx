import { FileLock2, HardDrive, ShieldCheck } from "lucide-react";
import { useI18n } from "../lib/i18n";

export default function PrivacyPage() {
  const { text } = useI18n();
  return (
    <section className="secondary-page">
      <div className="page-heading"><div><h1>{text("隐私说明", "Privacy")}</h1><p>{text("MetaClean 的处理边界清晰且可验证。", "MetaClean has clear, verifiable processing boundaries.")}</p></div></div>
      <div className="privacy-grid">
        <article><HardDrive size={21}/><h2>{text("纯本地运行", "Runs locally")}</h2><p>{text("扫描和清理均由本机 Rust 引擎完成，没有上传接口或云端处理。", "The local Rust engine scans and cleans files with no upload API or cloud processing.")}</p></article>
        <article><ShieldCheck size={21}/><h2>{text("先扫描后清理", "Scan before cleaning")}</h2><p>{text("扫描阶段只读。只有确认后才生成安全副本或替换原文件。", "Scanning is read-only. Files change only after explicit confirmation.")}</p></article>
        <article><FileLock2 size={21}/><h2>{text("处理你有权处理的内容", "Process authorized content")}</h2><p>{text("工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或处理未获授权的内容。", "Use this tool for privacy and file hygiene, not fraud, false provenance, or unauthorized content.")}</p></article>
      </div>
      <div className="scope-card"><h2>{text("当前支持范围", "Supported scope")}</h2><p>{text("JPEG、PNG、WebP 图片元数据；DOCX、XLSX、PPTX、ODT 文档属性及批注；PDF 属性和 XMP；TXT、Markdown、HTML、SVG 等文本中的不可见字符与生成器元数据。", "JPEG, PNG and WebP metadata; DOCX, XLSX, PPTX and ODT properties/comments; PDF properties and XMP; invisible Unicode and generator metadata in TXT, Markdown, HTML and SVG files.")}</p><p>{text("统计型文本水印、像素域水印、视频和传统二进制 Office 格式不在处理范围内。", "Statistical text watermarks, pixel-domain watermarks, video and legacy binary Office formats are out of scope.")}</p></div>
    </section>
  );
}
