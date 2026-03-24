<script lang="ts">
  import { t, onLocaleChange, getLocale } from '../lib/i18n';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  interface RoadmapItem {
    title: string;
    titleZh: string;
    desc: string;
    descZh: string;
    status: 'done' | 'in-progress' | 'planned';
  }

  const items: RoadmapItem[] = [
    // Done
    { title: 'Article publishing (Typst)', titleZh: '文章发布 (Typst)', desc: 'Create and publish articles with Typst format', descZh: '以 Typst 格式创建和发布文章', status: 'done' },
    { title: 'Fork & merge system', titleZh: 'Fork 和合并系统', desc: 'Fork articles and propose changes', descZh: 'Fork 文章并提交修改', status: 'done' },
    { title: 'Skill tree graph', titleZh: '技能树图', desc: 'Interactive skill tree visualization with compound nodes', descZh: '带有组合节点的交互式技能树可视化', status: 'done' },
    { title: 'Series (DAG lectures)', titleZh: '系列讲义 (DAG)', desc: 'Create lecture series with prerequisite-based ordering', descZh: '创建基于前置关系的系列讲义', status: 'done' },
    { title: 'Voting & bookmarks', titleZh: '投票与收藏', desc: 'Upvote/downvote and bookmark articles', descZh: '对文章投票和收藏', status: 'done' },
    { title: 'AT Protocol / PDS sync', titleZh: 'AT Protocol / PDS 同步', desc: 'Federated storage of articles, votes, and skill trees', descZh: '文章、投票和技能树的联邦化存储', status: 'done' },
    { title: 'i18n article translations', titleZh: '文章多语言', desc: 'Articles can have multiple language versions', descZh: '文章可以有多个语言版本', status: 'done' },
    { title: 'Keyboard shortcuts', titleZh: '快捷键导航', desc: 'Customizable keyboard shortcuts synced to PDS', descZh: '可自定义的快捷键，同步至 PDS', status: 'done' },
    { title: 'Chinese & English UI', titleZh: '中英双语界面', desc: 'Full site UI in Chinese and English', descZh: '整站中英文切换', status: 'done' },

    { title: 'License selection', titleZh: '协议选择', desc: 'Choose from multiple licenses (CC, MIT, Apache, etc.)', descZh: '支持多种授权协议（CC、MIT、Apache 等）', status: 'done' },
    { title: 'Profile links', titleZh: '个人链接', desc: 'Add links to personal website, Bluesky, GitHub, etc.', descZh: '添加个人网站、Bluesky、GitHub 等链接', status: 'done' },
    { title: 'Following & update feed', titleZh: '关注与更新流', desc: 'Follow users and see their updates on the homepage', descZh: '关注用户并在首页查看更新', status: 'done' },

    // In progress

    // Planned
    { title: 'Markdown + KaTeX support', titleZh: 'Markdown + KaTeX 支持', desc: 'Write articles in Markdown with KaTeX math rendering', descZh: '支持 Markdown 写作和 KaTeX 数学渲染', status: 'done' },
    { title: 'Pandoc format conversion', titleZh: 'Pandoc 格式转换', desc: 'Convert between Typst, Markdown, LaTeX, and more', descZh: '在 Typst、Markdown、LaTeX 等格式间转换', status: 'planned' },
    { title: 'Article editor', titleZh: '文章编辑器', desc: 'Edit published articles in-place', descZh: '在线编辑已发布文章', status: 'done' },
    { title: 'WYSIWYG editor', titleZh: '所见即所得编辑器', desc: 'Rich text editor with live preview for Markdown/Typst', descZh: '支持 Markdown/Typst 实时预览的富文本编辑器', status: 'planned' },
    { title: 'Image upload', titleZh: '图片上传', desc: 'Upload and embed images in articles', descZh: '在文章中上传和嵌入图片', status: 'planned' },
    { title: '.typ series upload', titleZh: '.typ 讲义批量上传', desc: 'Upload a .typ file and auto-split into series by heading', descZh: '上传 .typ 文件，按一级标题自动切分为系列', status: 'planned' },
    { title: 'PDS sync for skills', titleZh: '技能 PDS 同步', desc: 'Sync user skills and skill tree edges to PDS', descZh: '将用户技能和技能树同步至 PDS', status: 'planned' },
    { title: 'Learning state for skills', titleZh: '技能学习状态', desc: 'Mark skills as "learning" in addition to "mastered"', descZh: '技能支持"正在学习"状态', status: 'done' },
    { title: 'More languages', titleZh: '更多语言支持', desc: 'UI translations for Japanese, Korean, French, German, and more', descZh: '日语、韩语、法语、德语等更多界面语言', status: 'planned' },
    { title: 'SeriesDetail DAG view', titleZh: '系列详情 DAG 视图', desc: 'Visualize series articles as interactive DAG', descZh: '以交互式 DAG 可视化系列讲义', status: 'planned' },
    { title: 'CLI with AI upload', titleZh: 'CLI + AI 上传', desc: 'Local CLI tool that lets AI parse and upload your notes directly', descZh: '本地命令行工具，让 AI 解析并上传笔记到平台', status: 'planned' },
    { title: 'Paid articles', titleZh: '付费文章', desc: 'Support paid/premium articles with payment integration', descZh: '支持付费文章和支付集成', status: 'planned' },
  ];

  const statusOrder = { 'in-progress': 0, 'planned': 1, 'done': 2 };
  let sorted = $derived([...items].sort((a, b) => statusOrder[a.status] - statusOrder[b.status]));

  function statusLabel(s: string): string {
    if (s === 'done') return locale === 'zh' ? '已完成' : 'Done';
    if (s === 'in-progress') return locale === 'zh' ? '进行中' : 'In Progress';
    return locale === 'zh' ? '计划中' : 'Planned';
  }
</script>

<h1>{locale === 'zh' ? '路线图' : 'Roadmap'}</h1>
<p class="subtitle">{locale === 'zh' ? '计划实现的功能' : 'Planned features for Fedi-Xanadu'}</p>

<div class="roadmap">
  {#each sorted as item}
    <div class="roadmap-item status-{item.status}">
      <div class="item-status">
        <span class="status-dot"></span>
        <span class="status-text">{statusLabel(item.status)}</span>
      </div>
      <div class="item-content">
        <h3>{locale === 'zh' ? item.titleZh : item.title}</h3>
        <p>{locale === 'zh' ? item.descZh : item.desc}</p>
      </div>
    </div>
  {/each}
</div>

<style>
  h1 {
    margin-bottom: 0;
  }
  .subtitle {
    color: var(--text-secondary);
    font-size: 14px;
    margin: 4px 0 24px;
  }
  .roadmap {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .roadmap-item {
    display: flex;
    gap: 16px;
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    align-items: flex-start;
  }
  .item-status {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 90px;
    flex-shrink: 0;
    padding-top: 2px;
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .status-text {
    font-size: 12px;
    font-weight: 500;
  }
  .status-done .status-dot { background: #22c55e; }
  .status-done .status-text { color: #22c55e; }
  .status-in-progress .status-dot { background: #f59e0b; }
  .status-in-progress .status-text { color: #f59e0b; }
  .status-planned .status-dot { background: var(--text-hint); }
  .status-planned .status-text { color: var(--text-hint); }
  .item-content h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 500;
    color: var(--text-primary);
  }
  .item-content p {
    margin: 2px 0 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
