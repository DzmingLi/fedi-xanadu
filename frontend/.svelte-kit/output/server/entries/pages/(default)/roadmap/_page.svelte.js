import { c as escape_html, e as ensure_array_like, a as attr_class, b as stringify, j as derived } from "../../../../chunks/index2.js";
import { t, g as getLocale } from "../../../../chunks/index.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let locale = getLocale();
    const items = [
      // Done
      {
        title: {
          en: "Article publishing (Typst)",
          zh: "文章发布 (Typst)",
          fr: "Publication d'articles (Typst)"
        },
        desc: {
          en: "Create and publish articles with Typst format",
          zh: "以 Typst 格式创建和发布文章",
          fr: "Créer et publier des articles au format Typst"
        },
        status: "done"
      },
      {
        title: {
          en: "Fork & merge system",
          zh: "Fork 和合并系统",
          fr: "Système de fork et fusion"
        },
        desc: {
          en: "Fork articles and propose changes",
          zh: "Fork 文章并提交修改",
          fr: "Forker des articles et proposer des modifications"
        },
        status: "done"
      },
      {
        title: {
          en: "Skill tree graph",
          zh: "技能树图",
          fr: "Graphe d'arbre de compétences"
        },
        desc: {
          en: "Interactive skill tree visualization with compound nodes",
          zh: "带有组合节点的交互式技能树可视化",
          fr: "Visualisation interactive avec nœuds composés"
        },
        status: "done"
      },
      {
        title: {
          en: "Series (DAG lectures)",
          zh: "系列讲义 (DAG)",
          fr: "Séries de cours (DAG)"
        },
        desc: {
          en: "Create lecture series with prerequisite-based ordering",
          zh: "创建基于前置关系的系列讲义",
          fr: "Créer des séries avec ordonnancement par prérequis"
        },
        status: "done"
      },
      {
        title: {
          en: "Voting & bookmarks",
          zh: "投票与收藏",
          fr: "Votes et favoris"
        },
        desc: {
          en: "Upvote/downvote and bookmark articles",
          zh: "对文章投票和收藏",
          fr: "Voter pour et mettre en favori des articles"
        },
        status: "done"
      },
      {
        title: {
          en: "AT Protocol / PDS sync",
          zh: "AT Protocol / PDS 同步",
          fr: "Synchronisation AT Protocol / PDS"
        },
        desc: {
          en: "Federated storage of articles, votes, and skill trees",
          zh: "文章、投票和技能树的联邦化存储",
          fr: "Stockage fédéré des articles, votes et arbres de compétences"
        },
        status: "done"
      },
      {
        title: {
          en: "i18n article translations",
          zh: "文章多语言",
          fr: "Traductions d'articles"
        },
        desc: {
          en: "Articles can have multiple language versions",
          zh: "文章可以有多个语言版本",
          fr: "Les articles peuvent avoir plusieurs versions linguistiques"
        },
        status: "done"
      },
      {
        title: {
          en: "Keyboard shortcuts",
          zh: "快捷键导航",
          fr: "Raccourcis clavier"
        },
        desc: {
          en: "Customizable keyboard shortcuts synced to PDS",
          zh: "可自定义的快捷键，同步至 PDS",
          fr: "Raccourcis clavier personnalisables synchronisés vers PDS"
        },
        status: "done"
      },
      {
        title: {
          en: "Multilingual UI",
          zh: "多语言界面",
          fr: "Interface multilingue"
        },
        desc: {
          en: "Full site UI in Chinese, English, and French",
          zh: "整站中英法多语言切换",
          fr: "Interface complète en chinois, anglais et français"
        },
        status: "done"
      },
      {
        title: { en: "License selection", zh: "协议选择", fr: "Choix de licence" },
        desc: {
          en: "Choose from multiple licenses (CC, MIT, Apache, etc.)",
          zh: "支持多种授权协议（CC、MIT、Apache 等）",
          fr: "Choix parmi plusieurs licences (CC, MIT, Apache, etc.)"
        },
        status: "done"
      },
      {
        title: { en: "Profile links", zh: "个人链接", fr: "Liens de profil" },
        desc: {
          en: "Add links to personal website, Bluesky, GitHub, etc.",
          zh: "添加个人网站、Bluesky、GitHub 等链接",
          fr: "Ajouter des liens vers site personnel, Bluesky, GitHub, etc."
        },
        status: "done"
      },
      {
        title: {
          en: "Following & update feed",
          zh: "关注与更新流",
          fr: "Abonnements et fil d'actualité"
        },
        desc: {
          en: "Follow users and see their updates on the homepage",
          zh: "关注用户并在首页查看更新",
          fr: "Suivre des utilisateurs et voir leurs mises à jour"
        },
        status: "done"
      },
      {
        title: {
          en: "Markdown + MathML",
          zh: "Markdown + MathML 数学渲染",
          fr: "Markdown + MathML"
        },
        desc: {
          en: "Write articles in Markdown; LaTeX math formulas render as native MathML (no JS dependency)",
          zh: "支持 Markdown 写作，LaTeX 数学公式渲染为原生 MathML（无 JS 依赖）",
          fr: "Rédiger en Markdown avec rendu natif MathML (sans dépendance JS)"
        },
        status: "done"
      },
      {
        title: {
          en: "Series virtual-document rendering",
          zh: "系列虚拟文档渲染",
          fr: "Rendu de document virtuel pour les séries"
        },
        desc: {
          en: "Cross-chapter references resolve automatically — series chapters are compiled as a single document",
          zh: "跨章节引用自动解析——系列章节作为一个文档整体编译",
          fr: "Les références inter-chapitres sont résolues automatiquement"
        },
        status: "done"
      },
      {
        title: { en: "Article editor", zh: "文章编辑器", fr: "Éditeur d'articles" },
        desc: {
          en: "Edit published articles in-place",
          zh: "在线编辑已发布文章",
          fr: "Modifier les articles publiés sur place"
        },
        status: "done"
      },
      {
        title: {
          en: "Learning state for skills",
          zh: "技能学习状态",
          fr: "État d'apprentissage"
        },
        desc: {
          en: 'Mark skills as "learning" in addition to "mastered"',
          zh: '技能支持"正在学习"状态',
          fr: "Marquer les compétences comme « en cours » en plus de « maîtrisé »"
        },
        status: "done"
      },
      {
        title: {
          en: "User settings",
          zh: "用户设置",
          fr: "Paramètres utilisateur"
        },
        desc: {
          en: "Language preferences, default format, email, keybindings",
          zh: "语言偏好、默认格式、邮箱、快捷键设置",
          fr: "Préférences linguistiques, format par défaut, e-mail, raccourcis"
        },
        status: "done"
      },
      {
        title: {
          en: "Block & report",
          zh: "拉黑与举报",
          fr: "Blocage et signalement"
        },
        desc: {
          en: "Block users to hide their content; report users/articles for admin review",
          zh: "拉黑用户屏蔽其内容；举报用户/文章交由管理员处理",
          fr: "Bloquer des utilisateurs et signaler du contenu pour modération"
        },
        status: "done"
      },
      {
        title: {
          en: "Bookmark visibility",
          zh: "收藏夹可见性",
          fr: "Visibilité des favoris"
        },
        desc: {
          en: "Choose to make bookmarks public or share specific folders",
          zh: "选择公开收藏夹或仅分享特定文件夹",
          fr: "Choisir de rendre les favoris publics ou partager des dossiers spécifiques"
        },
        status: "done"
      },
      {
        title: {
          en: "Credentials verification",
          zh: "学历认证",
          fr: "Vérification des diplômes"
        },
        desc: {
          en: "Admin-verified education and affiliation displayed on profile",
          zh: "管理员认证学历、学位和单位，显示在个人主页",
          fr: "Éducation et affiliation vérifiées par l'admin, affichées sur le profil"
        },
        status: "done"
      },
      // Planned
      {
        title: {
          en: "Native Typst MathML",
          zh: "Typst 原生 MathML 支持",
          fr: "MathML natif Typst"
        },
        desc: {
          en: "When Typst adds native MathML output, we will adopt it directly — replacing the current mathyml shim with zero-overhead native rendering",
          zh: "当 Typst 原生支持 MathML 输出时，将直接采用，替换当前的 mathyml 中间层，实现零开销原生渲染",
          fr: "Lorsque Typst ajoutera la sortie MathML native, nous l'adopterons directement — remplaçant le shim mathyml actuel"
        },
        status: "planned"
      },
      {
        title: {
          en: "Pandoc format conversion",
          zh: "Pandoc 格式转换",
          fr: "Conversion de format Pandoc"
        },
        desc: {
          en: "Convert between Typst, Markdown, LaTeX, and more",
          zh: "在 Typst、Markdown、LaTeX 等格式间转换",
          fr: "Convertir entre Typst, Markdown, LaTeX et plus"
        },
        status: "planned"
      },
      {
        title: { en: "WYSIWYG editor", zh: "所见即所得编辑器", fr: "Éditeur WYSIWYG" },
        desc: {
          en: "Rich text editor with live preview for Markdown/Typst",
          zh: "支持 Markdown/Typst 实时预览的富文本编辑器",
          fr: "Éditeur riche avec aperçu en direct pour Markdown/Typst"
        },
        status: "planned"
      },
      {
        title: { en: "Image upload", zh: "图片上传", fr: "Import d'images" },
        desc: {
          en: "Upload and embed images in articles",
          zh: "在文章中上传和嵌入图片",
          fr: "Importer et intégrer des images dans les articles"
        },
        status: "planned"
      },
      {
        title: {
          en: ".typ series upload",
          zh: ".typ 讲义批量上传",
          fr: "Import de séries .typ"
        },
        desc: {
          en: "Upload a .typ file and auto-split into series by heading",
          zh: "上传 .typ 文件，按一级标题自动切分为系列",
          fr: "Importer un fichier .typ et le découper automatiquement par titres"
        },
        status: "planned"
      },
      {
        title: {
          en: "PDS sync for skills",
          zh: "技能 PDS 同步",
          fr: "Synchronisation PDS des compétences"
        },
        desc: {
          en: "Sync user skills and skill tree edges to PDS",
          zh: "将用户技能和技能树同步至 PDS",
          fr: "Synchroniser les compétences et arbres vers PDS"
        },
        status: "planned"
      },
      {
        title: { en: "More languages", zh: "更多语言支持", fr: "Plus de langues" },
        desc: {
          en: "UI translations for Japanese, Korean, German, and more",
          zh: "日语、韩语、德语等更多界面语言",
          fr: "Traductions pour le japonais, le coréen, l'allemand, etc."
        },
        status: "planned"
      },
      {
        title: {
          en: "SeriesDetail DAG view",
          zh: "系列详情 DAG 视图",
          fr: "Vue DAG des séries"
        },
        desc: {
          en: "Visualize series articles as interactive DAG",
          zh: "以交互式 DAG 可视化系列讲义",
          fr: "Visualiser les articles d'une série en DAG interactif"
        },
        status: "planned"
      },
      {
        title: {
          en: "CLI with AI upload",
          zh: "CLI + AI 上传",
          fr: "CLI avec import IA"
        },
        desc: {
          en: "Local CLI tool that lets AI parse and upload your notes directly",
          zh: "本地命令行工具，让 AI 解析并上传笔记到平台",
          fr: "Outil CLI local permettant à l'IA d'analyser et importer vos notes"
        },
        status: "planned"
      },
      {
        title: { en: "Paid articles", zh: "付费文章", fr: "Articles payants" },
        desc: {
          en: "Support paid/premium articles with payment integration",
          zh: "支持付费文章和支付集成",
          fr: "Articles payants avec intégration de paiement"
        },
        status: "planned"
      },
      {
        title: {
          en: "Campus email verification",
          zh: "校园邮箱验证",
          fr: "Vérification par e-mail universitaire"
        },
        desc: {
          en: "Verify school affiliation via campus email (currently admin-verified)",
          zh: "通过校园邮箱自动验证学校身份（目前由管理员认证）",
          fr: "Vérifier l'affiliation scolaire par e-mail universitaire (actuellement vérifié par l'admin)"
        },
        status: "planned"
      }
    ];
    const statusOrder = { "in-progress": 0, "planned": 1, "done": 2 };
    let sorted = derived(() => [...items].sort((a, b) => statusOrder[a.status] - statusOrder[b.status]));
    function loc(record) {
      return record[locale] || record["en"] || Object.values(record)[0];
    }
    $$renderer2.push(`<h1 class="svelte-1c1yhaw">${escape_html(t("roadmap.title"))}</h1> <p class="subtitle svelte-1c1yhaw">${escape_html(t("roadmap.subtitle"))}</p> <div class="roadmap svelte-1c1yhaw"><!--[-->`);
    const each_array = ensure_array_like(sorted());
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let item = each_array[$$index];
      $$renderer2.push(`<div${attr_class(`roadmap-item status-${stringify(item.status)}`, "svelte-1c1yhaw")}><div class="item-status svelte-1c1yhaw"><span class="status-dot svelte-1c1yhaw"></span> <span class="status-text svelte-1c1yhaw">${escape_html(t(`roadmap.${item.status === "in-progress" ? "inProgress" : item.status}`))}</span></div> <div class="item-content svelte-1c1yhaw"><h3 class="svelte-1c1yhaw">${escape_html(loc(item.title))}</h3> <p class="svelte-1c1yhaw">${escape_html(loc(item.desc))}</p></div></div>`);
    }
    $$renderer2.push(`<!--]--></div>`);
  });
}
export {
  _page as default
};
