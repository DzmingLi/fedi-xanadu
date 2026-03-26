<script lang="ts">
  import { t, getLocale, onLocaleChange } from '../lib/i18n';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  interface RoadmapItem {
    title: Record<string, string>;
    desc: Record<string, string>;
    status: 'done' | 'in-progress' | 'planned';
  }

  const items: RoadmapItem[] = [
    // Done
    { title: { en: 'Article publishing (Typst)', zh: '文章发布 (Typst)', fr: 'Publication d\'articles (Typst)' }, desc: { en: 'Create and publish articles with Typst format', zh: '以 Typst 格式创建和发布文章', fr: 'Créer et publier des articles au format Typst' }, status: 'done' },
    { title: { en: 'Fork & merge system', zh: 'Fork 和合并系统', fr: 'Système de fork et fusion' }, desc: { en: 'Fork articles and propose changes', zh: 'Fork 文章并提交修改', fr: 'Forker des articles et proposer des modifications' }, status: 'done' },
    { title: { en: 'Skill tree graph', zh: '技能树图', fr: 'Graphe d\'arbre de compétences' }, desc: { en: 'Interactive skill tree visualization with compound nodes', zh: '带有组合节点的交互式技能树可视化', fr: 'Visualisation interactive avec nœuds composés' }, status: 'done' },
    { title: { en: 'Series (DAG lectures)', zh: '系列讲义 (DAG)', fr: 'Séries de cours (DAG)' }, desc: { en: 'Create lecture series with prerequisite-based ordering', zh: '创建基于前置关系的系列讲义', fr: 'Créer des séries avec ordonnancement par prérequis' }, status: 'done' },
    { title: { en: 'Voting & bookmarks', zh: '投票与收藏', fr: 'Votes et favoris' }, desc: { en: 'Upvote/downvote and bookmark articles', zh: '对文章投票和收藏', fr: 'Voter pour et mettre en favori des articles' }, status: 'done' },
    { title: { en: 'AT Protocol / PDS sync', zh: 'AT Protocol / PDS 同步', fr: 'Synchronisation AT Protocol / PDS' }, desc: { en: 'Federated storage of articles, votes, and skill trees', zh: '文章、投票和技能树的联邦化存储', fr: 'Stockage fédéré des articles, votes et arbres de compétences' }, status: 'done' },
    { title: { en: 'i18n article translations', zh: '文章多语言', fr: 'Traductions d\'articles' }, desc: { en: 'Articles can have multiple language versions', zh: '文章可以有多个语言版本', fr: 'Les articles peuvent avoir plusieurs versions linguistiques' }, status: 'done' },
    { title: { en: 'Keyboard shortcuts', zh: '快捷键导航', fr: 'Raccourcis clavier' }, desc: { en: 'Customizable keyboard shortcuts synced to PDS', zh: '可自定义的快捷键，同步至 PDS', fr: 'Raccourcis clavier personnalisables synchronisés vers PDS' }, status: 'done' },
    { title: { en: 'Multilingual UI', zh: '多语言界面', fr: 'Interface multilingue' }, desc: { en: 'Full site UI in Chinese, English, and French', zh: '整站中英法多语言切换', fr: 'Interface complète en chinois, anglais et français' }, status: 'done' },
    { title: { en: 'License selection', zh: '协议选择', fr: 'Choix de licence' }, desc: { en: 'Choose from multiple licenses (CC, MIT, Apache, etc.)', zh: '支持多种授权协议（CC、MIT、Apache 等）', fr: 'Choix parmi plusieurs licences (CC, MIT, Apache, etc.)' }, status: 'done' },
    { title: { en: 'Profile links', zh: '个人链接', fr: 'Liens de profil' }, desc: { en: 'Add links to personal website, Bluesky, GitHub, etc.', zh: '添加个人网站、Bluesky、GitHub 等链接', fr: 'Ajouter des liens vers site personnel, Bluesky, GitHub, etc.' }, status: 'done' },
    { title: { en: 'Following & update feed', zh: '关注与更新流', fr: 'Abonnements et fil d\'actualité' }, desc: { en: 'Follow users and see their updates on the homepage', zh: '关注用户并在首页查看更新', fr: 'Suivre des utilisateurs et voir leurs mises à jour' }, status: 'done' },
    { title: { en: 'Markdown + KaTeX support', zh: 'Markdown + KaTeX 支持', fr: 'Support Markdown + KaTeX' }, desc: { en: 'Write articles in Markdown with KaTeX math rendering', zh: '支持 Markdown 写作和 KaTeX 数学渲染', fr: 'Rédiger en Markdown avec rendu mathématique KaTeX' }, status: 'done' },
    { title: { en: 'Article editor', zh: '文章编辑器', fr: 'Éditeur d\'articles' }, desc: { en: 'Edit published articles in-place', zh: '在线编辑已发布文章', fr: 'Modifier les articles publiés sur place' }, status: 'done' },
    { title: { en: 'Learning state for skills', zh: '技能学习状态', fr: 'État d\'apprentissage' }, desc: { en: 'Mark skills as "learning" in addition to "mastered"', zh: '技能支持"正在学习"状态', fr: 'Marquer les compétences comme « en cours » en plus de « maîtrisé »' }, status: 'done' },

    // Planned
    { title: { en: 'Pandoc format conversion', zh: 'Pandoc 格式转换', fr: 'Conversion de format Pandoc' }, desc: { en: 'Convert between Typst, Markdown, LaTeX, and more', zh: '在 Typst、Markdown、LaTeX 等格式间转换', fr: 'Convertir entre Typst, Markdown, LaTeX et plus' }, status: 'planned' },
    { title: { en: 'WYSIWYG editor', zh: '所见即所得编辑器', fr: 'Éditeur WYSIWYG' }, desc: { en: 'Rich text editor with live preview for Markdown/Typst', zh: '支持 Markdown/Typst 实时预览的富文本编辑器', fr: 'Éditeur riche avec aperçu en direct pour Markdown/Typst' }, status: 'planned' },
    { title: { en: 'Image upload', zh: '图片上传', fr: 'Import d\'images' }, desc: { en: 'Upload and embed images in articles', zh: '在文章中上传和嵌入图片', fr: 'Importer et intégrer des images dans les articles' }, status: 'planned' },
    { title: { en: '.typ series upload', zh: '.typ 讲义批量上传', fr: 'Import de séries .typ' }, desc: { en: 'Upload a .typ file and auto-split into series by heading', zh: '上传 .typ 文件，按一级标题自动切分为系列', fr: 'Importer un fichier .typ et le découper automatiquement par titres' }, status: 'planned' },
    { title: { en: 'PDS sync for skills', zh: '技能 PDS 同步', fr: 'Synchronisation PDS des compétences' }, desc: { en: 'Sync user skills and skill tree edges to PDS', zh: '将用户技能和技能树同步至 PDS', fr: 'Synchroniser les compétences et arbres vers PDS' }, status: 'planned' },
    { title: { en: 'More languages', zh: '更多语言支持', fr: 'Plus de langues' }, desc: { en: 'UI translations for Japanese, Korean, German, and more', zh: '日语、韩语、德语等更多界面语言', fr: 'Traductions pour le japonais, le coréen, l\'allemand, etc.' }, status: 'planned' },
    { title: { en: 'SeriesDetail DAG view', zh: '系列详情 DAG 视图', fr: 'Vue DAG des séries' }, desc: { en: 'Visualize series articles as interactive DAG', zh: '以交互式 DAG 可视化系列讲义', fr: 'Visualiser les articles d\'une série en DAG interactif' }, status: 'planned' },
    { title: { en: 'CLI with AI upload', zh: 'CLI + AI 上传', fr: 'CLI avec import IA' }, desc: { en: 'Local CLI tool that lets AI parse and upload your notes directly', zh: '本地命令行工具，让 AI 解析并上传笔记到平台', fr: 'Outil CLI local permettant à l\'IA d\'analyser et importer vos notes' }, status: 'planned' },
    { title: { en: 'Paid articles', zh: '付费文章', fr: 'Articles payants' }, desc: { en: 'Support paid/premium articles with payment integration', zh: '支持付费文章和支付集成', fr: 'Articles payants avec intégration de paiement' }, status: 'planned' },
  ];

  const statusOrder = { 'in-progress': 0, 'planned': 1, 'done': 2 };
  let sorted = $derived([...items].sort((a, b) => statusOrder[a.status] - statusOrder[b.status]));

  function loc(record: Record<string, string>): string {
    return record[locale] || record['en'] || Object.values(record)[0];
  }
</script>

<h1>{t('roadmap.title')}</h1>
<p class="subtitle">{t('roadmap.subtitle')}</p>

<div class="roadmap">
  {#each sorted as item}
    <div class="roadmap-item status-{item.status}">
      <div class="item-status">
        <span class="status-dot"></span>
        <span class="status-text">{t(`roadmap.${item.status === 'in-progress' ? 'inProgress' : item.status}`)}</span>
      </div>
      <div class="item-content">
        <h3>{loc(item.title)}</h3>
        <p>{loc(item.desc)}</p>
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
