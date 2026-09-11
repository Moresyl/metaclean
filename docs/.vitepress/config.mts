import { defineConfig } from "vitepress";

export default defineConfig({
  lang: "zh-CN",
  title: "MetaClean",
  description: "本地优先、可验证、失败关闭的文件隐私清理工具",
  cleanUrls: true,
  lastUpdated: true,
  srcExclude: ["**/.tmp-*/**", "**/.firecrawl/**"],
  themeConfig: {
    logo: "/logo.svg",
    siteTitle: "MetaClean",
    nav: [
      { text: "文档", link: "/" },
      { text: "用户指南", link: "/user-guide" },
      { text: "架构", link: "/ARCHITECTURE" },
      { text: "路线与证据", link: "/PLAN" },
      { text: "GitHub", link: "https://github.com/Moresyl/metaclean" },
    ],
    sidebar: {
      "/": [
        {
          text: "开始",
          items: [
            { text: "文档中心", link: "/" },
            { text: "产品能力", link: "/product" },
            { text: "用户指南", link: "/user-guide" },
          ],
        },
        {
          text: "工程事实",
          items: [
            { text: "架构", link: "/ARCHITECTURE" },
            { text: "路线图", link: "/PLAN" },
            { text: "竞品审计", link: "/competitive-audit" },
            { text: "设计系统", link: "/design" },
          ],
        },
        {
          text: "交付",
          items: [
            { text: "贡献与发布", link: "/release" },
            { text: "验证账本", link: "/validation" },
            { text: "安全与支持策略", link: "/safety" },
            { text: "安全报告", link: "/security" },
            { text: "支持策略", link: "/support-policy" },
            { text: "变更记录", link: "/changelog" },
          ],
        },
      ],
    },
    socialLinks: [{ icon: "github", link: "https://github.com/Moresyl/metaclean" }],
    search: { provider: "local" },
    outline: { level: [2, 3] },
    docFooter: { prev: "上一页", next: "下一页" },
    editLink: { pattern: "https://github.com/Moresyl/metaclean/edit/master/docs/:path" },
  },
});
