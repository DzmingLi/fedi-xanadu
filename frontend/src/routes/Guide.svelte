<script lang="ts">
</script>

<article class="guide">
  <h1>Fedi-Xanadu 使用指南</h1>

  <section>
    <h2>什么是 Fedi-Xanadu？</h2>
    <p>Fedi-Xanadu 是一个基于 <a href="https://atproto.com">AT Protocol</a> 的去中心化知识分享平台。</p>
    <ul>
      <li><strong>前置知识感知</strong> — 每篇文章标注所需前置知识，帮助读者判断是否准备好阅读</li>
      <li><strong>技能树系统</strong> — 社区共建知识图谱，浏览、fork、投票选出最佳学习路径</li>
      <li><strong>系列讲义</strong> — 将多篇文章组织成有序课程，支持章节间的前置依赖</li>
      <li><strong>书架（书签系统）</strong> — 收藏文章到自定义文件夹，打造个人知识库</li>
      <li><strong>草稿箱</strong> — 随时保存未完成的文章，草稿同步到你的 PDS</li>
      <li><strong>去中心化</strong> — 你的文章存储在你自己的 AT Protocol PDS 上，不依赖单一平台</li>
      <li><strong>Fork 机制</strong> — 像代码一样 fork 文章，社区择优</li>
    </ul>
  </section>

  <section>
    <h2>核心概念</h2>

    <h3>Tag（标签）</h3>
    <p>Tag 是知识的最小单位标识，如 <code>calculus</code>、<code>linear-algebra</code>。任何人都可以自由创建 tag。</p>
    <p>Tag 用于：</p>
    <ul>
      <li>标记文章所属领域</li>
      <li>声明文章的前置知识要求</li>
      <li>构建技能树中的节点</li>
    </ul>

    <h3>技能树（Skill Tree）</h3>
    <p>技能树是 tag 之间的层级关系图谱。每个人都可以创建自己的技能树，也可以 fork 别人的。</p>
    <p>社区投票决定哪些技能树最有价值 — 高赞技能树自然涌现为推荐学习路径。</p>
    <p>你可以「采用」一棵技能树，它将成为你的活跃知识图谱，帮助你追踪学习进度。</p>

    <h3>前置知识（Prerequisites）</h3>
    <p>文章可以声明三种级别的前置知识：</p>
    <ul>
      <li><strong class="required">必需</strong> — 不理解这些概念将无法阅读本文</li>
      <li><strong class="recommended">推荐</strong> — 了解这些会有更好的阅读体验</li>
      <li><strong class="suggested">建议</strong> — 有所了解即可，不是硬性要求</li>
    </ul>

    <h3>系列讲义（Series）</h3>
    <p>多篇文章可以组织为系列讲义。系列讲义：</p>
    <ul>
      <li>有明确的阅读顺序（章节编号）</li>
      <li>章节间可以建立直接的前置依赖关系</li>
      <li>阅读文章时左右侧有导航箭头跳转前后章节</li>
    </ul>

    <h3>书架（书签系统）</h3>
    <p>收藏你喜欢的文章，组织成个人知识库：</p>
    <ul>
      <li>点击文章页面的「收藏」按钮，将文章加入书架</li>
      <li>创建自定义文件夹分类整理书签</li>
      <li>在「Library」页面管理你的所有收藏</li>
    </ul>

    <h3>草稿箱（Drafts）</h3>
    <p>写作过程中可以随时保存草稿：</p>
    <ul>
      <li>在编辑器中点击「存为草稿」，内容会保存到服务器和你的 PDS</li>
      <li>草稿可以在「草稿」页面查看、继续编辑或发布</li>
      <li>发布草稿时，文章会自动创建并同步到 AT Protocol 网络</li>
    </ul>
  </section>

  <section>
    <h2>用 Typst 写作</h2>
    <p>Fedi-Xanadu 使用 <a href="https://typst.app">Typst</a> 作为内容格式 — 一种现代的排版语言，特别适合数学和学术写作。</p>

    <h3>基础语法</h3>
    <pre><code>= 一级标题
== 二级标题
=== 三级标题

普通段落文本。*粗体*、_斜体_、`行内代码`。

- 无序列表项
- 另一项
  - 嵌套

+ 有序列表
+ 第二项</code></pre>

    <h3>数学公式</h3>
    <pre><code>行内公式：$f(x) = x^2 + 1$

独立公式块：
$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $

矩阵：
$ mat(1, 2; 3, 4) $

求和：
$ sum_(n=1)^infinity 1/n^2 = pi^2/6 $</code></pre>

    <h3>定理环境</h3>
    <pre><code>#let theorem = block.with(
  stroke: luma(200),
  inset: 12pt,
  radius: 4pt,
)

#theorem[
  *定理 (欧拉公式)* \
  $e^(i pi) + 1 = 0$
]</code></pre>

    <h3>脚注 → 边注</h3>
    <p>Typst 的脚注会自动转换为侧边边注（sidenotes），在宽屏上显示在正文右侧：</p>
    <pre><code>这是正文内容#footnote[这条脚注会显示为边注]。</code></pre>

    <h3>图片</h3>
    <pre><code>#figure(
  image("diagram.png", width: 80%),
  caption: [图 1: 示意图],
)</code></pre>

    <h3>代码块</h3>
    <pre><code>```rust
fn main() &lbrace;
    println!("Hello, world!");
&rbrace;
```</code></pre>
  </section>

  <section>
    <h2>快速开始</h2>
    <ol>
      <li>使用 Bluesky 账号登录（Handle + App Password）</li>
      <li>选择感兴趣的领域，首页将按领域推荐文章</li>
      <li>浏览<a href="#/skill-trees">技能树</a>，采用一棵适合你的学习路径</li>
      <li>点击「Write」<a href="#/new">创建文章</a>，用 Typst 编写内容</li>
      <li>为文章添加 tag 和前置知识声明</li>
      <li>发布后，文章会同时存储在你的 AT Protocol PDS 上</li>
    </ol>
  </section>
</article>

<style>
  .guide {
    max-width: 680px;
    line-height: 1.7;
  }
  .guide h1 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin-bottom: 1.5rem;
  }
  .guide h2 {
    font-family: var(--font-serif);
    font-weight: 400;
    font-size: 1.4rem;
    margin: 2rem 0 0.5rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--border);
  }
  .guide h3 {
    font-family: var(--font-serif);
    font-weight: 500;
    font-size: 1.1rem;
    margin: 1.2rem 0 0.3rem;
  }
  .guide p {
    margin: 0.5rem 0;
    font-size: 15px;
  }
  .guide ul, .guide ol {
    margin: 0.3rem 0;
    padding-left: 1.5rem;
  }
  .guide li {
    margin: 0.2rem 0;
    font-size: 15px;
  }
  .guide pre {
    background: var(--bg-gray, #f6f6f6);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 12px 16px;
    overflow-x: auto;
    font-size: 13px;
    line-height: 1.5;
  }
  .guide code {
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
  }
  .guide a { color: var(--accent); }
  .required { color: #c33; }
  .recommended { color: #b8860b; }
  .suggested { color: var(--accent); }
</style>
