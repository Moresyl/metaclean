import { FileLock2, HardDrive, ShieldCheck } from "lucide-react";
import { useI18n } from "../lib/i18n";

export default function PrivacyPage() {
  const { text } = useI18n();
  return (
    <section className="secondary-page">
      <div className="privacy-grid">
        <article><HardDrive size={21}/><h2>{text("文件纯本地处理", "Files stay local")}</h2><p>{text("扫描和清理均由本机 Rust 引擎完成，没有文件上传接口或云端处理。启用版本检查时只请求 GitHub Releases。", "The local Rust engine scans and cleans files with no upload API or cloud processing. Update checks only request GitHub Releases.")}</p></article>
        <article><ShieldCheck size={21}/><h2>{text("先扫描后清理", "Scan before cleaning")}</h2><p>{text("扫描阶段只读。只有确认后才生成安全副本或替换原文件。", "Scanning is read-only. Files change only after explicit confirmation.")}</p></article>
        <article><FileLock2 size={21}/><h2>{text("处理你有权处理的内容", "Process authorized content")}</h2><p>{text("工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或处理未获授权的内容。", "Use this tool for privacy and file hygiene, not fraud, false provenance, or unauthorized content.")}</p></article>
      </div>
      <div className="scope-card"><h2>{text("当前支持范围", "Supported scope")}</h2><p>{text("91 种扩展名：JPEG、PNG、WebP、GIF、BMP、TIFF、HEIC/HEIF、AVIF 及 23 种相机 RAW 格式的图片元数据；MP3、WAV、FLAC、WMA、M4A 音频标签；19 种 MP4/QuickTime、AVI、ASF/WMV 与 Matroska/WebM 容器中的用户数据、XMP 与位置；DOCX、XLSX、PPTX、ODT、EPUB 文档属性及批注；PDF 属性和 XMP；16 种 UTF-8 文本与标记文件中的不可见字符及适用的生成器元数据。", "91 extensions: metadata in JPEG, PNG, WebP, GIF, BMP, TIFF, HEIC/HEIF, AVIF and 23 camera raw formats; MP3, WAV, FLAC, WMA and M4A audio tags; user data, XMP and location in 19 MP4/QuickTime, AVI, ASF/WMV and Matroska/WebM containers; DOCX, XLSX, PPTX, ODT and EPUB properties and comments; PDF properties and XMP; invisible Unicode and applicable generator metadata in 16 UTF-8 text and markup formats.")}</p><p>{text("统计型文本水印、像素域水印和传统二进制 Office 格式不在处理范围内。", "Statistical text watermarks, pixel-domain watermarks and legacy binary Office formats are out of scope.")}</p></div>
    </section>
  );
}
