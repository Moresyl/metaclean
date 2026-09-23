import { defineConfig } from "vitepress";

export default defineConfig({
  base: process.env.DOCS_BASE ?? "/",
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
      { text: "开始", link: "/" },
      { text: "用户指南", link: "/user-guide" },
      { text: "安全模型", link: "/safety" },
      { text: "架构", link: "/ARCHITECTURE" },
      { text: "验证账本", link: "/validation" },
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
    search: {
      provider: "local",
      options: {
        translations: {
          button: { buttonText: "搜索", buttonAriaLabel: "搜索文档" },
          modal: {
            displayDetails: "显示详细结果",
            resetButtonTitle: "清除搜索",
            backButtonTitle: "返回",
            noResultsText: "没有找到结果",
            footer: {
              selectText: "选择",
              selectKeyAriaLabel: "回车键",
              navigateText: "导航",
              navigateUpKeyAriaLabel: "向上箭头",
              navigateDownKeyAriaLabel: "向下箭头",
              closeText: "关闭",
              closeKeyAriaLabel: "Esc 键",
            },
          },
        },
      },
    },
    outline: { level: [2, 3], label: "本页导航" },
    lastUpdated: { text: "最后更新" },
    editLink: { pattern: "https://github.com/Moresyl/metaclean/edit/master/docs/:path", text: "编辑此页" },
    darkModeSwitchLabel: "切换主题",
    lightModeSwitchTitle: "切换到浅色模式",
    darkModeSwitchTitle: "切换到深色模式",
    sidebarMenuLabel: "打开侧边栏",
    returnToTopLabel: "返回顶部",
    langMenuLabel: "切换语言",
    skipToContentLabel: "跳转到正文",
    docFooter: { prev: "上一页", next: "下一页" },
    notFound: {
      code: "404",
      title: "页面不存在",
      quote: "这条路径没有文档。页面可能已移动，但你的文件从未离开本机。",
      linkLabel: "回到文档中心",
      linkText: "回到文档中心",
    },
    footer: {
      message: "事实优先 · 本地处理 · 失败关闭",
      copyright: "MetaClean · MIT License",
    },
  },
});
