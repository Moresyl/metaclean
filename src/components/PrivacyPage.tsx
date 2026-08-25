import { FileLock2, HardDrive, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import { useI18n } from "../lib/i18n";

export default function PrivacyPage() {
  const { text } = useI18n();
  return (
    <section className="grid h-full max-w-[900px] auto-rows-max gap-3 overflow-y-auto pr-0.5">
      <div className="grid gap-3 sm:grid-cols-3">
        <Claim icon={<HardDrive size={17} strokeWidth={1.8} />} title={text("文件纯本地处理", "Files stay local")}>
          {text("扫描和清理均由本机 Rust 引擎完成，没有文件上传接口或云端处理。启用版本检查时只请求 GitHub Releases。", "The local Rust engine scans and cleans files with no upload API or cloud processing. Update checks only request GitHub Releases.")}
        </Claim>
        <Claim icon={<ShieldCheck size={17} strokeWidth={1.8} />} title={text("先扫描后清理", "Scan before cleaning")}>
          {text("扫描阶段只读。只有确认后才生成安全副本或替换原文件。", "Scanning is read-only. Files change only after explicit confirmation.")}
        </Claim>
        <Claim icon={<FileLock2 size={17} strokeWidth={1.8} />} title={text("处理你有权处理的内容", "Process authorized content")}>
          {text("工具用于隐私和文件卫生，不应用于学术欺诈、伪造来源或处理未获授权的内容。", "Use this tool for privacy and file hygiene, not fraud, false provenance, or unauthorized content.")}
        </Claim>
      </div>

      <div className="selectable grid gap-2 rounded-panel border border-line bg-surface p-4 shadow-panel">
        <h2 className="text-md font-semibold">{text("当前支持范围", "Supported scope")}</h2>
        <p className="text-sm leading-relaxed text-muted">
          {text("105 种扩展名：JPEG、PNG、WebP/JPEG XL、GIF、BMP、TIFF、HEIC/HEIF、AVIF 及 23 种相机 RAW 格式的图片元数据；MP3、WAV、FLAC/AIFF、WMA、M4A 音频标签；19 种 MP4/QuickTime、AVI、ASF/WMV 与 Matroska/WebM 容器中的用户数据、XMP 与位置；DOCX、XLSX、PPTX、ODF、EPUB 文档属性及批注；PDF 属性和 XMP；16 种 UTF-8 文本与标记文件中的不可见字符及适用的生成器元数据。", "105 extensions: metadata in JPEG, PNG, WebP/JPEG XL, GIF, BMP, TIFF, HEIC/HEIF, AVIF and 23 camera raw formats; MP3, WAV, FLAC/AIFF, WMA and M4A audio tags; user data, XMP and location in 19 MP4/QuickTime, AVI, ASF/WMV and Matroska/WebM containers; DOCX, XLSX, PPTX, ODF and EPUB properties and comments; PDF properties and XMP; invisible Unicode and applicable generator metadata in 16 UTF-8 text and markup formats.")}
        </p>
        {/* Kept beside the scope rather than on a page of its own: the limits are
            part of the claim, and a claim whose limits are elsewhere is a boast. */}
        {/* Same ink as the scope above it, deliberately. Setting the limits a
            shade dimmer than the claim is the typographic version of small
            print, and the whole point of the paragraph is that this app does
            not do that. The hairline says it is a separate thought; that is
            enough. */}
        <p className="border-t border-line pt-2 text-sm leading-relaxed text-muted">
          {text("统计型文本水印、像素域水印和传统二进制 Office 格式不在处理范围内。", "Statistical text watermarks, pixel-domain watermarks and legacy binary Office formats are out of scope.")}
        </p>
      </div>
    </section>
  );
}

function Claim({ icon, title, children }: { icon: ReactNode; title: string; children: ReactNode }) {
  return (
    <article className="grid content-start gap-1.5 rounded-panel border border-line bg-surface p-3.5 shadow-panel">
      <span className="grid size-8 place-items-center rounded-control bg-brand/12 text-brand" aria-hidden="true">{icon}</span>
      <h2 className="text-base font-semibold">{title}</h2>
      <p className="text-sm leading-relaxed text-muted">{children}</p>
    </article>
  );
}
