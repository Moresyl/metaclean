import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Button from "./Button";
import { useI18n } from "../lib/i18n";

interface Preview { fields: { source: string; name: string; value: string }[]; truncated: boolean }

/** Values live only in this mounted, explicitly opened panel. */
export default function MetadataPreview({ path, disabled }: { path: string; disabled: boolean }) {
  const { text } = useI18n();
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState<Preview>();
  const [failed, setFailed] = useState(false);
  const propertyName = (name: string) => {
    const key = name.split("/").find(part => /(^|:)(creator|author|title|subject|description|keywords|lastModifiedBy|created|modified|Company|Manager|Application|generator)$/i.test(part))?.split(":").pop()?.toLowerCase();
    const labels: Record<string, string> = {
      creator: text("作者", "Author"), author: text("作者", "Author"), title: text("标题", "Title"),
      subject: text("主题", "Subject"), description: text("描述", "Description"), keywords: text("关键词", "Keywords"),
      lastmodifiedby: text("最后修改者", "Last modified by"), created: text("创建时间", "Created"), modified: text("修改时间", "Modified"),
      company: text("公司", "Company"), manager: text("管理者", "Manager"), application: text("应用程序", "Application"), generator: text("生成器", "Generator"),
    };
    return key && labels[key] ? `${labels[key]} · ${name}` : name;
  };
  useEffect(() => {
    setPreview(undefined);
    setFailed(false);
    if (disabled) setOpen(false);
    if (!open || disabled) return;
    let current = true;
    void invoke<Preview>("preview_metadata", { path }).then(result => {
      if (current) setPreview(result);
    }).catch(() => { if (current) setFailed(true); });
    return () => { current = false; };
  }, [open, disabled, path]);
  const visible = open && !disabled;
  return <section className="grid gap-2 rounded-control border border-line bg-surface p-2.5">
    <Button variant="ghost" disabled={disabled} aria-expanded={visible} onClick={() => {
      setPreview(undefined);
      setFailed(false);
      setOpen(!open);
    }}>{visible ? text("隐藏文档属性", "Hide document properties") : text("查看文档属性内容", "View document property values")}</Button>
    {visible ? <>
      <p className="text-xs text-muted">{text("当前文件的只读预览，可能包含个人信息。关闭后清除，不写入历史或审计报告。仅显示文档属性，不代表全部清理项，也不选择性删除字段。", "Read-only preview of the current file; it may contain personal information. Dismissed values are not saved in history or audit reports. Document properties are a subset of cleanup findings; this does not select individual fields for removal.")}</p>
      {failed ? <p role="alert">{text("无法读取属性，文件可能已变更、损坏或不受支持。请重新扫描后重试。", "Could not read properties. The file may have changed, be damaged or unsupported. Scan again and retry.")}</p>
        : !preview ? <p role="status">{text("正在读取属性…", "Reading properties…")}</p>
        : <>
          {!preview.fields.length ? <p role="status">{text("当前文件未发现可预览的文档属性。这不代表不存在其他隐私痕迹。", "No previewable document properties found in the current file. Other privacy traces may still exist.")}</p> : null}
          <dl className="grid max-h-72 gap-2 overflow-auto text-xs">
            {preview.fields.map((field, index) => <div key={index} className="min-w-0 border-b border-line pb-2">
              <dt className="break-all font-medium">{propertyName(field.name)} <span className="font-normal text-muted">· {field.source}</span></dt>
              <dd className="selectable whitespace-pre-wrap break-all" dir="auto">{field.value}</dd>
            </div>)}
          </dl>
          {preview.truncated ? <p role="status" className="text-muted">{text("内容较多，仅预览前 128 项，每项最多 2048 个字符。", "Preview limited to 128 fields and 2048 characters per value.")}</p> : null}
        </>}
    </> : null}
  </section>;
}
