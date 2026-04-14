import{k as l,v as t}from"./index-DWniHu3i.js";import"./legacy-DjtBTLYc.js";var o=t(`<article class="guide svelte-18nxouq"><h1 class="svelte-18nxouq">NightBoat 使用指南</h1> <section><h2 class="svelte-18nxouq">什么是 NightBoat？</h2> <p class="svelte-18nxouq">NightBoat 是一个基于 <a href="https://atproto.com" class="svelte-18nxouq">AT Protocol</a> 的去中心化知识分享平台。</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq"><strong>前置知识感知</strong> — 每篇文章标注所需前置知识，帮助读者判断是否准备好阅读</li> <li class="svelte-18nxouq"><strong>技能树系统</strong> — 社区共建知识图谱，浏览、fork、投票选出最佳学习路径</li> <li class="svelte-18nxouq"><strong>系列讲义</strong> — 将多篇文章组织成有序课程，支持章节间的前置依赖</li> <li class="svelte-18nxouq"><strong>书架（书签系统）</strong> — 收藏文章到自定义文件夹，打造个人知识库</li> <li class="svelte-18nxouq"><strong>草稿箱</strong> — 随时保存未完成的文章，草稿同步到你的 PDS</li> <li class="svelte-18nxouq"><strong>去中心化</strong> — 你的文章存储在你自己的 AT Protocol PDS 上，不依赖单一平台</li> <li class="svelte-18nxouq"><strong>Fork 机制</strong> — 像代码一样 fork 文章，社区择优</li></ul></section> <section><h2 class="svelte-18nxouq">核心概念</h2> <h3 class="svelte-18nxouq">Tag（标签）</h3> <p class="svelte-18nxouq">Tag 是知识的最小单位标识，如 <code class="svelte-18nxouq">calculus</code>、<code class="svelte-18nxouq">linear-algebra</code>。任何人都可以自由创建 tag。</p> <p class="svelte-18nxouq">Tag 用于：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq">标记文章所属领域</li> <li class="svelte-18nxouq">声明文章的前置知识要求</li> <li class="svelte-18nxouq">构建技能树中的节点</li></ul> <h3 class="svelte-18nxouq">技能树（Skill Tree）</h3> <p class="svelte-18nxouq">技能树是 tag 之间的层级关系图谱。每个人都可以创建自己的技能树，也可以 fork 别人的。</p> <p class="svelte-18nxouq">社区投票决定哪些技能树最有价值 — 高赞技能树自然涌现为推荐学习路径。</p> <p class="svelte-18nxouq">你可以「采用」一棵技能树，它将成为你的活跃知识图谱，帮助你追踪学习进度。</p> <h3 class="svelte-18nxouq">前置知识（Prerequisites）</h3> <p class="svelte-18nxouq">文章可以声明三种级别的前置知识：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq"><strong class="required svelte-18nxouq">必需</strong> — 不理解这些概念将无法阅读本文</li> <li class="svelte-18nxouq"><strong class="recommended svelte-18nxouq">推荐</strong> — 了解这些会有更好的阅读体验</li> <li class="svelte-18nxouq"><strong class="suggested svelte-18nxouq">建议</strong> — 有所了解即可，不是硬性要求</li></ul> <h3 class="svelte-18nxouq">系列讲义（Series）</h3> <p class="svelte-18nxouq">系列讲义是一个完整的 <strong>Typst 项目</strong>（一个 pijul 仓库），包含 <code class="svelte-18nxouq">main.typ</code> 入口文件和所有章节文件、参考文献等共享资源。</p> <p class="svelte-18nxouq">系统编译 <code class="svelte-18nxouq">main.typ</code>，然后按照你选择的标题级别自动拆分成独立页面，每个页面有自己的 URL 和评论区。</p> <h4>项目结构</h4> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">my-series/
├── main.typ          ← 入口文件（必须）
├── chapters/
│   ├── intro.typ
│   ├── ch1.typ
│   ├── ch2.typ
│   └── ...
├── references.bib    ← 参考文献（可选）
├── images/           ← 图片目录（可选）
└── macros.typ        ← 自定义宏（可选）</code></pre> <h4>main.typ 示例</h4> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">#set heading(numbering: "1.1")

#include "chapters/intro.typ"
#include "chapters/ch1.typ"
#include "chapters/ch2.typ"

#bibliography("references.bib")</code></pre> <p class="svelte-18nxouq">发布时选择拆分级别：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq"><strong>按一级标题拆分</strong> — 每个 <code class="svelte-18nxouq">= Chapter</code> 是一个独立页面</li> <li class="svelte-18nxouq"><strong>按二级标题拆分</strong> — 每个 <code class="svelte-18nxouq">== Section</code> 是一个独立页面（适合细粒度内容如教材）</li></ul> <p class="svelte-18nxouq">例如，《Composing Programs》按二级标题拆分后的结构：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">Composing Programs
├── Chapter 1: Building Abstractions with Functions
│   ├── 1.1 Getting Started（页面，独立评论区）
│   ├── 1.2 Elements of Programming（页面）
│   └── 1.3 Defining New Functions（页面）
├── Chapter 2: Building Abstractions with Data
│   ├── 2.1 Introduction（页面）
│   └── ...
└── ...</code></pre> <h4>跨章引用和参考文献</h4> <p class="svelte-18nxouq">因为整个系列作为一个文档编译，Typst 的跨文件引用自然生效：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">// 在 ch1.typ 中定义标签
= 类型论基础 &lt;ch:type-theory&gt;

// 在 ch2.typ 中引用
如 @ch:type-theory 所述...

// 参考文献引用
根据 Martin-Löf @Martin-Lof-1972 的工作...</code></pre> <h4>Fork 系列</h4> <p class="svelte-18nxouq">Fork 一个系列 = fork 整个仓库。你可以修改任何章节，添加新内容，系统会跟踪 pijul diff。</p> <h3 class="svelte-18nxouq">书架（书签系统）</h3> <p class="svelte-18nxouq">收藏你喜欢的文章，组织成个人知识库：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq">点击文章页面的「收藏」按钮，将文章加入书架</li> <li class="svelte-18nxouq">创建自定义文件夹分类整理书签</li> <li class="svelte-18nxouq">在「Library」页面管理你的所有收藏</li></ul> <h3 class="svelte-18nxouq">草稿箱（Drafts）</h3> <p class="svelte-18nxouq">写作过程中可以随时保存草稿：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq">在编辑器中点击「存为草稿」，内容会保存到服务器和你的 PDS</li> <li class="svelte-18nxouq">草稿可以在「草稿」页面查看、继续编辑或发布</li> <li class="svelte-18nxouq">发布草稿时，文章会自动创建并同步到 AT Protocol 网络</li></ul></section> <section><h2 class="svelte-18nxouq">内容格式</h2> <p class="svelte-18nxouq">平台支持三种写作格式：<strong>Typst</strong>（推荐）、<strong>Markdown</strong>、<strong>HTML</strong>。上传文件时根据扩展名自动识别（<code class="svelte-18nxouq">.typ</code>、<code class="svelte-18nxouq">.md</code>、<code class="svelte-18nxouq">.html</code>）。</p> <p class="svelte-18nxouq">系列讲义推荐使用 Typst（跨章引用、参考文献支持最完整），也支持 Markdown 和 HTML。</p></section> <section><h2 class="svelte-18nxouq">Typst（推荐）</h2> <p class="svelte-18nxouq"><a href="https://typst.app" class="svelte-18nxouq">Typst</a> 是现代排版语言，特别适合数学和学术写作。平台默认格式。</p> <h3 class="svelte-18nxouq">基础语法</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">= 一级标题
== 二级标题
=== 三级标题

普通段落文本。*粗体*、_斜体_、\`行内代码\`。

- 无序列表项
- 另一项
  - 嵌套

+ 有序列表
+ 第二项</code></pre> <h3 class="svelte-18nxouq">数学公式</h3> <p class="svelte-18nxouq">数学渲染为 MathML，浏览器原生支持，无需额外加载。</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">行内公式：$f(x) = x^2 + 1$

独立公式块：
$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $

矩阵：
$ mat(1, 2; 3, 4) $

求和：
$ sum_(n=1)^infinity 1/n^2 = pi^2/6 $</code></pre> <h3 class="svelte-18nxouq">内置定理环境</h3> <p class="svelte-18nxouq">平台预置了一组学术环境，直接使用即可，自动带样式：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">#definition(name: "连续")[
  设 $f: RR -> RR$，若 $lim_(x -> a) f(x) = f(a)$，
  则称 $f$ 在 $a$ 点连续。
]

#theorem[若 $f$ 在 $[a, b]$ 上连续，则 $f$ 可积。]

#proof[显然。]

#lemma[引理内容]
#corollary[推论内容]
#proposition[命题内容]
#remark[注记内容]
#example[示例内容]
#solution[解答内容]</code></pre> <h3 class="svelte-18nxouq">脚注 → 边注</h3> <p class="svelte-18nxouq">Typst 的脚注会自动转换为侧边边注（sidenotes），在宽屏上显示在正文右侧：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">这是正文内容#footnote[这条脚注会显示为边注]。</code></pre> <h3 class="svelte-18nxouq">图片</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">#figure(
  image("diagram.png", width: 80%),
  caption: [图 1: 示意图],
)</code></pre> <h3 class="svelte-18nxouq">代码块</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">\`\`\`rust
fn main() &lbrace;
    println!("Hello, world!");
&rbrace;
\`\`\`</code></pre></section> <section><h2 class="svelte-18nxouq">Markdown</h2> <p class="svelte-18nxouq">支持 CommonMark 扩展语法，适合熟悉 Markdown 的用户。</p> <h3 class="svelte-18nxouq">数学公式</h3> <p class="svelte-18nxouq">使用 LaTeX 语法，服务端渲染为 MathML：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">行内公式：$x^2 + y^2 = r^2$

块级公式：
$$
\\int_a^b f(x)\\,dx = F(b) - F(a)
$$</code></pre> <h3 class="svelte-18nxouq">定理环境</h3> <p class="svelte-18nxouq">使用 callout 语法（类似 Obsidian），在引用块中以 <code class="svelte-18nxouq">[!type]</code> 开头：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">&gt; [!theorem] 勾股定理
&gt; 对于直角三角形，$a^2 + b^2 = c^2$。

&gt; [!proof]
&gt; 由相似三角形可得...

&gt; [!definition] 连续
&gt; 函数 $f$ 在 $x_0$ 点连续...</code></pre> <p class="svelte-18nxouq">支持的类型：<code class="svelte-18nxouq">theorem</code>、<code class="svelte-18nxouq">lemma</code>、<code class="svelte-18nxouq">corollary</code>、<code class="svelte-18nxouq">proposition</code>、<code class="svelte-18nxouq">definition</code>、<code class="svelte-18nxouq">proof</code>、<code class="svelte-18nxouq">remark</code>、<code class="svelte-18nxouq">example</code>、<code class="svelte-18nxouq">solution</code>。</p> <p class="svelte-18nxouq">渲染效果与 Typst 的定理环境完全一致。</p> <h3 class="svelte-18nxouq">扩展语法</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq"># 标题

**加粗** 和 *斜体*

| 列A | 列B |
|-----|-----|
| 1   | 2   |

- [x] 任务列表
- [ ] 未完成

脚注引用[^1]

[^1]: 脚注内容

~~删除线~~</code></pre> <h3 class="svelte-18nxouq">图片</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">![描述](filename.png)</code></pre></section> <section></section> <section><h2 class="svelte-18nxouq">HTML</h2> <p class="svelte-18nxouq">直接提供渲染好的 HTML 内容片段。</p> <h3 class="svelte-18nxouq">重要：只提交内容片段</h3> <p class="svelte-18nxouq">HTML 文章必须是<strong>内容片段</strong>（fragment），不是完整的 HTML 页面。</p> <div class="do-dont svelte-18nxouq"><div class="do svelte-18nxouq"><h4 class="svelte-18nxouq">正确</h4> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;h2&gt;1.2 Elements of Programming&lt;/h2&gt;
&lt;p&gt;Every powerful language has three mechanisms...&lt;/p&gt;
&lt;ul&gt;
  &lt;li&gt;primitive expressions&lt;/li&gt;
  &lt;li&gt;means of combination&lt;/li&gt;
&lt;/ul&gt;</code></pre></div> <div class="dont svelte-18nxouq"><h4 class="svelte-18nxouq">错误</h4> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;!DOCTYPE html&gt;
&lt;html&gt;
&lt;head&gt;
  &lt;link rel="stylesheet" href="..."&gt;
  &lt;script src="..."&gt;&lt;/script&gt;
&lt;/head&gt;
&lt;body&gt;
  &lt;h2&gt;1.2 Elements...&lt;/h2&gt;
&lt;/body&gt;
&lt;/html&gt;</code></pre></div></div> <p class="svelte-18nxouq"><strong>不允许</strong>以下标签：</p> <ul class="svelte-18nxouq"><li class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;!DOCTYPE&gt;</code>、<code class="svelte-18nxouq">&lt;html&gt;</code>、<code class="svelte-18nxouq">&lt;head&gt;</code>、<code class="svelte-18nxouq">&lt;body&gt;</code> — 页面结构由平台提供</li> <li class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;script&gt;</code> — 出于安全考虑，不允许执行脚本</li> <li class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;link rel="stylesheet"&gt;</code> — 样式由平台统一提供</li></ul> <h3 class="svelte-18nxouq">嵌入视频</h3> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;iframe
  width="640" height="360"
  src="https://www.youtube.com/embed/VIDEO_ID?rel=0"
  frameborder="0"
  allowfullscreen
  style="max-width: 100%;"
&gt;&lt;/iframe&gt;</code></pre> <h3 class="svelte-18nxouq">图片</h3> <p class="svelte-18nxouq">先通过编辑器上传图片，然后使用返回的路径：</p> <pre class="svelte-18nxouq"><code class="svelte-18nxouq">&lt;img src="/api/articles/image?uri=YOUR_URI&amp;filename=diagram.png"
     alt="示意图" style="max-width: 100%;"&gt;</code></pre></section> <section><h2 class="svelte-18nxouq">快速开始</h2> <ol class="svelte-18nxouq"><li class="svelte-18nxouq">使用 Bluesky 账号登录（Handle + App Password）</li> <li class="svelte-18nxouq">选择感兴趣的领域，首页将按领域推荐文章</li> <li class="svelte-18nxouq">浏览<a href="/skills" class="svelte-18nxouq">技能树</a>，采用一棵适合你的学习路径</li> <li class="svelte-18nxouq">点击「Write」<a href="/new" class="svelte-18nxouq">创建文章</a>，选择 Typst / Markdown / HTML 格式</li> <li class="svelte-18nxouq">为文章添加 tag 和前置知识声明</li> <li class="svelte-18nxouq">发布后，文章会同时存储在你的 AT Protocol PDS 上</li></ol></section></article>`);function a(s){var e=o();l(s,e)}export{a as default};
