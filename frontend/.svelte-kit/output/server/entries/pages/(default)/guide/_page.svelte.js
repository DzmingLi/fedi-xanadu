import "clsx";
function _page($$renderer) {
  $$renderer.push(`<article class="guide svelte-vpxp7s"><h1 class="svelte-vpxp7s">Fedi-Xanadu 使用指南</h1> <section><h2 class="svelte-vpxp7s">什么是 Fedi-Xanadu？</h2> <p class="svelte-vpxp7s">Fedi-Xanadu 是一个基于 <a href="https://atproto.com" class="svelte-vpxp7s">AT Protocol</a> 的去中心化知识分享平台。</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s"><strong>前置知识感知</strong> — 每篇文章标注所需前置知识，帮助读者判断是否准备好阅读</li> <li class="svelte-vpxp7s"><strong>技能树系统</strong> — 社区共建知识图谱，浏览、fork、投票选出最佳学习路径</li> <li class="svelte-vpxp7s"><strong>系列讲义</strong> — 将多篇文章组织成有序课程，支持章节间的前置依赖</li> <li class="svelte-vpxp7s"><strong>书架（书签系统）</strong> — 收藏文章到自定义文件夹，打造个人知识库</li> <li class="svelte-vpxp7s"><strong>草稿箱</strong> — 随时保存未完成的文章，草稿同步到你的 PDS</li> <li class="svelte-vpxp7s"><strong>去中心化</strong> — 你的文章存储在你自己的 AT Protocol PDS 上，不依赖单一平台</li> <li class="svelte-vpxp7s"><strong>Fork 机制</strong> — 像代码一样 fork 文章，社区择优</li></ul></section> <section><h2 class="svelte-vpxp7s">核心概念</h2> <h3 class="svelte-vpxp7s">Tag（标签）</h3> <p class="svelte-vpxp7s">Tag 是知识的最小单位标识，如 <code class="svelte-vpxp7s">calculus</code>、<code class="svelte-vpxp7s">linear-algebra</code>。任何人都可以自由创建 tag。</p> <p class="svelte-vpxp7s">Tag 用于：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">标记文章所属领域</li> <li class="svelte-vpxp7s">声明文章的前置知识要求</li> <li class="svelte-vpxp7s">构建技能树中的节点</li></ul> <h3 class="svelte-vpxp7s">技能树（Skill Tree）</h3> <p class="svelte-vpxp7s">技能树是 tag 之间的层级关系图谱。每个人都可以创建自己的技能树，也可以 fork 别人的。</p> <p class="svelte-vpxp7s">社区投票决定哪些技能树最有价值 — 高赞技能树自然涌现为推荐学习路径。</p> <p class="svelte-vpxp7s">你可以「采用」一棵技能树，它将成为你的活跃知识图谱，帮助你追踪学习进度。</p> <h3 class="svelte-vpxp7s">前置知识（Prerequisites）</h3> <p class="svelte-vpxp7s">文章可以声明三种级别的前置知识：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s"><strong class="required svelte-vpxp7s">必需</strong> — 不理解这些概念将无法阅读本文</li> <li class="svelte-vpxp7s"><strong class="recommended svelte-vpxp7s">推荐</strong> — 了解这些会有更好的阅读体验</li> <li class="svelte-vpxp7s"><strong class="suggested svelte-vpxp7s">建议</strong> — 有所了解即可，不是硬性要求</li></ul> <h3 class="svelte-vpxp7s">系列讲义（Series）</h3> <p class="svelte-vpxp7s">多篇文章可以组织为系列讲义。系列讲义支持<strong>多级嵌套</strong>，适合结构化教材：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">有明确的阅读顺序（章节编号）</li> <li class="svelte-vpxp7s">章节间可以建立直接的前置依赖关系</li> <li class="svelte-vpxp7s">阅读文章时左右侧有导航箭头跳转前后章节</li> <li class="svelte-vpxp7s"><strong>嵌套结构</strong> — 创建子系列作为章节，子系列下再添加文章作为小节</li></ul> <p class="svelte-vpxp7s">例如，上传《Composing Programs》教材的推荐结构：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">Composing Programs（根系列）
├── Chapter 1: Building Abstractions with Functions（子系列）
│   ├── 1.1 Getting Started（文章）
│   ├── 1.2 Elements of Programming（文章）
│   └── 1.3 Defining New Functions（文章）
├── Chapter 2: Building Abstractions with Data（子系列）
│   ├── 2.1 Introduction（文章）
│   └── ...
└── ...</code></pre> <p class="svelte-vpxp7s">每篇文章内部用标题（h2/h3）组织小节内容，例如 1.2 文章内含 1.2.1 ~ 1.2.6 等小节。</p> <h3 class="svelte-vpxp7s">书架（书签系统）</h3> <p class="svelte-vpxp7s">收藏你喜欢的文章，组织成个人知识库：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">点击文章页面的「收藏」按钮，将文章加入书架</li> <li class="svelte-vpxp7s">创建自定义文件夹分类整理书签</li> <li class="svelte-vpxp7s">在「Library」页面管理你的所有收藏</li></ul> <h3 class="svelte-vpxp7s">草稿箱（Drafts）</h3> <p class="svelte-vpxp7s">写作过程中可以随时保存草稿：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">在编辑器中点击「存为草稿」，内容会保存到服务器和你的 PDS</li> <li class="svelte-vpxp7s">草稿可以在「草稿」页面查看、继续编辑或发布</li> <li class="svelte-vpxp7s">发布草稿时，文章会自动创建并同步到 AT Protocol 网络</li></ul></section> <section><h2 class="svelte-vpxp7s">内容格式</h2> <p class="svelte-vpxp7s">平台支持四种写作格式，在编辑器的「格式」下拉菜单中选择。上传文件时会根据扩展名自动识别。</p> <p class="svelte-vpxp7s">Fork 文章时可以切换目标格式，系统会自动转换内容。转换可能丢失部分格式特有的语法（如 Typst 定理环境转 Markdown 后变为普通文本），建议转换后检查。</p></section> <section><h2 class="svelte-vpxp7s">Typst（推荐）</h2> <p class="svelte-vpxp7s"><a href="https://typst.app" class="svelte-vpxp7s">Typst</a> 是现代排版语言，特别适合数学和学术写作。平台默认格式。</p> <h3 class="svelte-vpxp7s">基础语法</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">= 一级标题
== 二级标题
=== 三级标题

普通段落文本。*粗体*、_斜体_、\`行内代码\`。

- 无序列表项
- 另一项
  - 嵌套

+ 有序列表
+ 第二项</code></pre> <h3 class="svelte-vpxp7s">数学公式</h3> <p class="svelte-vpxp7s">数学渲染为 MathML，浏览器原生支持，无需额外加载。</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">行内公式：$f(x) = x^2 + 1$

独立公式块：
$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $

矩阵：
$ mat(1, 2; 3, 4) $

求和：
$ sum_(n=1)^infinity 1/n^2 = pi^2/6 $</code></pre> <h3 class="svelte-vpxp7s">内置定理环境</h3> <p class="svelte-vpxp7s">平台预置了一组学术环境，直接使用即可，自动带样式：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">#definition(name: "连续")[
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
#solution[解答内容]</code></pre> <h3 class="svelte-vpxp7s">脚注 → 边注</h3> <p class="svelte-vpxp7s">Typst 的脚注会自动转换为侧边边注（sidenotes），在宽屏上显示在正文右侧：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">这是正文内容#footnote[这条脚注会显示为边注]。</code></pre> <h3 class="svelte-vpxp7s">图片</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">#figure(
  image("diagram.png", width: 80%),
  caption: [图 1: 示意图],
)</code></pre> <h3 class="svelte-vpxp7s">代码块</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`</code></pre></section> <section><h2 class="svelte-vpxp7s">Markdown</h2> <p class="svelte-vpxp7s">支持 CommonMark 扩展语法，适合熟悉 Markdown 的用户。</p> <h3 class="svelte-vpxp7s">数学公式</h3> <p class="svelte-vpxp7s">使用 KaTeX 语法，服务端渲染：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">行内公式：$x^2 + y^2 = r^2$

块级公式：
$$
\\int_a^b f(x)\\,dx = F(b) - F(a)
$$</code></pre> <h3 class="svelte-vpxp7s">扩展语法</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s"># 标题

**加粗** 和 *斜体*

| 列A | 列B |
|-----|-----|
| 1   | 2   |

- [x] 任务列表
- [ ] 未完成

脚注引用[^1]

[^1]: 脚注内容

~~删除线~~</code></pre> <h3 class="svelte-vpxp7s">图片</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">![描述](filename.png)</code></pre></section> <section><h2 class="svelte-vpxp7s">LaTeX</h2> <p class="svelte-vpxp7s">适合已有 LaTeX 文档或习惯 LaTeX 语法的用户。通过 pandoc 转换为 HTML，数学渲染为 MathML。</p> <h3 class="svelte-vpxp7s">基本用法</h3> <p class="svelte-vpxp7s">不需要 <code class="svelte-vpxp7s">\\documentclass</code> 和 <code class="svelte-vpxp7s">\\begin{document}</code>，直接写正文内容：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">\\section{引言}

这是一段文字，包含行内公式 $E = mc^2$ 和块级公式：
\\[
  \\int_a^b f(x)\\,dx = F(b) - F(a)
\\]

\\subsection{列表}

\\begin{enumerate}
\\item 第一项
\\item 第二项
\\end{enumerate}</code></pre> <h3 class="svelte-vpxp7s">支持的命令</h3> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">章节：<code class="svelte-vpxp7s">\\section</code>、<code class="svelte-vpxp7s">\\subsection</code>、<code class="svelte-vpxp7s">\\subsubsection</code></li> <li class="svelte-vpxp7s">格式：<code class="svelte-vpxp7s">\\textbf{}</code>、<code class="svelte-vpxp7s">\\textit{}</code>、<code class="svelte-vpxp7s">\\emph{}</code></li> <li class="svelte-vpxp7s">列表：<code class="svelte-vpxp7s">enumerate</code>、<code class="svelte-vpxp7s">itemize</code></li> <li class="svelte-vpxp7s">数学：<code class="svelte-vpxp7s">$...$</code>、<code class="svelte-vpxp7s">\\[...\\]</code>、<code class="svelte-vpxp7s">equation</code>、<code class="svelte-vpxp7s">align</code></li> <li class="svelte-vpxp7s">环境：<code class="svelte-vpxp7s">theorem</code>、<code class="svelte-vpxp7s">proof</code>、<code class="svelte-vpxp7s">definition</code> 等</li></ul> <h3 class="svelte-vpxp7s">注意事项</h3> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s">不支持自定义宏包（<code class="svelte-vpxp7s">\\usepackage</code>），只处理标准 LaTeX</li> <li class="svelte-vpxp7s">复杂表格和 TikZ 图形可能无法正确转换</li> <li class="svelte-vpxp7s">建议先在本地编译确认效果后再上传</li></ul></section> <section><h2 class="svelte-vpxp7s">HTML</h2> <p class="svelte-vpxp7s">直接提供渲染好的 HTML 内容片段。</p> <h3 class="svelte-vpxp7s">重要：只提交内容片段</h3> <p class="svelte-vpxp7s">HTML 文章必须是<strong>内容片段</strong>（fragment），不是完整的 HTML 页面。</p> <div class="do-dont svelte-vpxp7s"><div class="do svelte-vpxp7s"><h4 class="svelte-vpxp7s">正确</h4> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;h2>1.2 Elements of Programming&lt;/h2>
&lt;p>Every powerful language has three mechanisms...&lt;/p>
&lt;ul>
  &lt;li>primitive expressions&lt;/li>
  &lt;li>means of combination&lt;/li>
&lt;/ul></code></pre></div> <div class="dont svelte-vpxp7s"><h4 class="svelte-vpxp7s">错误</h4> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;!DOCTYPE html>
&lt;html>
&lt;head>
  &lt;link rel="stylesheet" href="...">
  &lt;script src="...">&lt;/script>
&lt;/head>
&lt;body>
  &lt;h2>1.2 Elements...&lt;/h2>
&lt;/body>
&lt;/html></code></pre></div></div> <p class="svelte-vpxp7s"><strong>不允许</strong>以下标签：</p> <ul class="svelte-vpxp7s"><li class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;!DOCTYPE></code>、<code class="svelte-vpxp7s">&lt;html></code>、<code class="svelte-vpxp7s">&lt;head></code>、<code class="svelte-vpxp7s">&lt;body></code> — 页面结构由平台提供</li> <li class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;script></code> — 出于安全考虑，不允许执行脚本</li> <li class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;link rel="stylesheet"></code> — 样式由平台统一提供</li></ul> <h3 class="svelte-vpxp7s">嵌入视频</h3> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;iframe
  width="640" height="360"
  src="https://www.youtube.com/embed/VIDEO_ID?rel=0"
  frameborder="0"
  allowfullscreen
  style="max-width: 100%;"
>&lt;/iframe></code></pre> <h3 class="svelte-vpxp7s">图片</h3> <p class="svelte-vpxp7s">先通过编辑器上传图片，然后使用返回的路径：</p> <pre class="svelte-vpxp7s"><code class="svelte-vpxp7s">&lt;img src="/api/articles/image?uri=YOUR_URI&amp;filename=diagram.png"
     alt="示意图" style="max-width: 100%;"></code></pre></section> <section><h2 class="svelte-vpxp7s">快速开始</h2> <ol class="svelte-vpxp7s"><li class="svelte-vpxp7s">使用 Bluesky 账号登录（Handle + App Password）</li> <li class="svelte-vpxp7s">选择感兴趣的领域，首页将按领域推荐文章</li> <li class="svelte-vpxp7s">浏览<a href="/skill-trees" class="svelte-vpxp7s">技能树</a>，采用一棵适合你的学习路径</li> <li class="svelte-vpxp7s">点击「Write」<a href="/new" class="svelte-vpxp7s">创建文章</a>，选择 Typst / Markdown / LaTeX / HTML 格式</li> <li class="svelte-vpxp7s">为文章添加 tag 和前置知识声明</li> <li class="svelte-vpxp7s">发布后，文章会同时存储在你的 AT Protocol PDS 上</li></ol></section></article>`);
}
export {
  _page as default
};
