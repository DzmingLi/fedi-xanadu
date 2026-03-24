const STORAGE_KEY = 'fx_locale';

type Locale = 'zh' | 'en';

let _locale: Locale = (localStorage.getItem(STORAGE_KEY) as Locale) || 'zh';
let _listeners: Array<() => void> = [];

export function getLocale(): Locale {
  return _locale;
}

export function setLocale(locale: Locale) {
  _locale = locale;
  localStorage.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale;
  _listeners.forEach(fn => fn());
}

export function onLocaleChange(fn: () => void): () => void {
  _listeners.push(fn);
  return () => { _listeners = _listeners.filter(f => f !== fn); };
}

export function t(key: string): string {
  return messages[_locale]?.[key] ?? messages['en']?.[key] ?? key;
}

const messages: Record<Locale, Record<string, string>> = {
  zh: {
    // Nav
    'nav.skills': '技能树',
    'nav.library': '书架',
    'nav.about': '关于',
    'nav.roadmap': '路线图',
    'nav.newArticle': '写文章',
    'nav.newSeries': '创建系列',
    'nav.search': '搜索文章...',
    'nav.login': '登录',
    'nav.logout': '退出',

    // Home
    'home.trending': '热门',
    'home.recent': '最新文章',
    'home.all': '全部',
    'home.noArticles': '该栏目暂无内容',
    'home.writeOne': '写一篇',
    'home.selectInterests': '选择你感兴趣的领域',
    'home.interestHint': '选择后首页将按领域分栏展示文章，随时可在设置中修改',
    'home.confirm': '确认',
    'home.fields': '个领域',
    'home.series': '系列',
    'home.lectures': '篇讲义',
    'home.votes': '赞数',
    'home.bookmarks': '收藏数',
    'home.following': '关注',

    // Sidebar
    'sidebar.home': '首页',
    'sidebar.skills': '技能树',
    'sidebar.library': '书架',
    'sidebar.guide': '指南',
    'sidebar.about': '关于',
    'sidebar.desc': '基于 AT Protocol 的联邦化知识共享平台，前置知识感知内容匹配。',
    'sidebar.learnMore': '了解更多',

    // Article
    'article.prereqs': '前置知识',
    'article.forks': '分叉',
    'article.translations': '其他语言版本',
    'article.bookmark': '收藏',
    'article.bookmarked': '已收藏',
    'article.fork': 'Fork',
    'article.upvote': '赞',
    'article.downvote': '踩',
    'article.loginToVote': '登录后可投票',
    'article.series': '所属系列',
    'article.loading': '加载中...',
    'article.error': '错误',
    'article.edit': '编辑',
    'article.delete': '删除',
    'article.deleteConfirm': '确定要删除这篇文章吗？此操作不可撤销。',
    'article.comments': '评论',
    'article.noComments': '暂无评论',
    'article.writeComment': '写评论...',
    'article.submit': '发表',
    'article.loginToComment': '登录后可评论',

    // NewArticle
    'newArticle.title': '新文章',
    'newArticle.titleLabel': '标题',
    'newArticle.descLabel': '描述',
    'newArticle.descPlaceholder': '首页预览的简短描述',
    'newArticle.langLabel': '语言',
    'newArticle.licenseLabel': '许可证',
    'newArticle.translationOf': '翻译自（可选）',
    'newArticle.originalArticle': '— 原创文章 —',
    'newArticle.formatLabel': '格式',
    'newArticle.contentLabel': '正文',
    'newArticle.tagsLabel': '标签（点击选择）',
    'newArticle.prereqsLabel': '前置知识',
    'newArticle.prereqsHint': '读者需要掌握哪些前置知识才能理解本文？',
    'newArticle.selectTag': '选择标签...',
    'newArticle.required': '必须',
    'newArticle.recommended': '推荐',
    'newArticle.suggested': '建议',
    'newArticle.addPrereq': '添加',
    'newArticle.publishing': '发布中...',
    'newArticle.publish': '发布',

    // Keybindings
    'kb.title': '快捷键',
    'kb.customize': '自定义',
    'kb.resetAll': '重置全部',
    'kb.save': '保存并关闭',
    'kb.close': '关闭',
    'kb.syncHint': '登录后可将快捷键同步至 PDS',

    // Roadmap
    'roadmap.title': '路线图',
    'roadmap.subtitle': '计划实现的功能',
    'roadmap.planned': '计划中',
    'roadmap.inProgress': '进行中',
    'roadmap.done': '已完成',

    // Generic
    'search.noResults': '无结果',
  },
  en: {
    // Nav
    'nav.skills': 'Skill Tree',
    'nav.library': 'Library',
    'nav.about': 'About',
    'nav.roadmap': 'Roadmap',
    'nav.newArticle': 'Write',
    'nav.newSeries': 'New Series',
    'nav.search': 'Search articles...',
    'nav.login': 'Login',
    'nav.logout': 'Logout',

    // Home
    'home.trending': 'Trending',
    'home.recent': 'Recent Articles',
    'home.all': 'All',
    'home.noArticles': 'No content yet',
    'home.writeOne': 'Write one',
    'home.selectInterests': 'Select your interests',
    'home.interestHint': 'Articles will be organized by topic. You can change this later.',
    'home.confirm': 'Confirm',
    'home.fields': 'topics',
    'home.series': 'Series',
    'home.lectures': 'lectures',
    'home.votes': 'Votes',
    'home.bookmarks': 'Bookmarks',
    'home.following': 'Following',

    // Sidebar
    'sidebar.home': 'Home',
    'sidebar.skills': 'Skill Tree',
    'sidebar.library': 'Library',
    'sidebar.guide': 'Guide',
    'sidebar.about': 'About',
    'sidebar.desc': 'Federated knowledge sharing on AT Protocol. Prereq-aware content matching.',
    'sidebar.learnMore': 'Learn more',

    // Article
    'article.prereqs': 'Prerequisites',
    'article.forks': 'Forks',
    'article.translations': 'Other languages',
    'article.bookmark': 'Bookmark',
    'article.bookmarked': 'Bookmarked',
    'article.fork': 'Fork',
    'article.upvote': 'Upvote',
    'article.downvote': 'Downvote',
    'article.loginToVote': 'Log in to vote',
    'article.series': 'In series',
    'article.loading': 'Loading...',
    'article.error': 'Error',
    'article.edit': 'Edit',
    'article.delete': 'Delete',
    'article.deleteConfirm': 'Are you sure you want to delete this article? This cannot be undone.',
    'article.comments': 'Comments',
    'article.noComments': 'No comments yet',
    'article.writeComment': 'Write a comment...',
    'article.submit': 'Submit',
    'article.loginToComment': 'Log in to comment',

    // NewArticle
    'newArticle.title': 'New Article',
    'newArticle.titleLabel': 'Title',
    'newArticle.descLabel': 'Description',
    'newArticle.descPlaceholder': 'Brief description for homepage preview',
    'newArticle.langLabel': 'Language',
    'newArticle.licenseLabel': 'License',
    'newArticle.translationOf': 'Translation of (optional)',
    'newArticle.originalArticle': '— original article —',
    'newArticle.formatLabel': 'Format',
    'newArticle.contentLabel': 'Content',
    'newArticle.tagsLabel': 'Tags (click to select)',
    'newArticle.prereqsLabel': 'Prerequisites',
    'newArticle.prereqsHint': 'What prior knowledge does a reader need to understand this article?',
    'newArticle.selectTag': 'Select tag...',
    'newArticle.required': 'Required',
    'newArticle.recommended': 'Recommended',
    'newArticle.suggested': 'Suggested',
    'newArticle.addPrereq': 'Add',
    'newArticle.publishing': 'Publishing...',
    'newArticle.publish': 'Publish',

    // Keybindings
    'kb.title': 'Keyboard Shortcuts',
    'kb.customize': 'Customize',
    'kb.resetAll': 'Reset all',
    'kb.save': 'Save & Close',
    'kb.close': 'Close',
    'kb.syncHint': 'Log in to sync shortcuts to PDS',

    // Roadmap
    'roadmap.title': 'Roadmap',
    'roadmap.subtitle': 'Planned features',
    'roadmap.planned': 'Planned',
    'roadmap.inProgress': 'In Progress',
    'roadmap.done': 'Done',

    // Generic
    'search.noResults': 'No results',
  },
};
