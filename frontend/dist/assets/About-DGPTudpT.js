import{p as o,k as e,l as r,v as d}from"./index-BblCZ0Pl.js";import"./legacy-X5fwNhVO.js";import{h as s}from"./html-Bc7i3c7a.js";import{g as u,i as p}from"./marked.esm-CnkQlpD7.js";const k=`# Fedi-Xanadu：让知识流向对的人

## 你有没有遇到过这样的情况？

在知乎上刷到一篇写得极好的文章，讲 Ludics 的，但你发现自己看不懂——因为你不会 Linear Logic。你收藏了，想着以后再看，但"以后"永远没有来。

反过来也是：你写了一篇很用心的编译原理文章，结果推给了一堆不知道什么是上下文无关文法的人，收获的只有"看不懂"。

**好内容推给没有前置知识的人是浪费，而有前置知识的人又找不到好内容。** 这就是 Fedi-Xanadu 要解决的问题。

## Fedi-Xanadu 是什么

一句话：**一个基于前置知识匹配的联邦式知识共享平台。**

名字来自两个致敬：Ted Nelson 的 [Xanadu Project](https://zhuanlan.zhihu.com/p/59394667)——互联网本应成为的样子；以及 Ward Cunningham 的 [Federated Wiki](http://fed.wiki.org/view/federated-wiki)——一个天才的想法，只是从未被好好实现过。

## 三个核心设计

### 1. 技能树：你会什么，我就给你推什么

平台上有一棵由社区共建的「技能树」，本质上是一系列话题标签（Tag）。你可以点亮你已经掌握的 Tag——纯自我声明，就像游戏里点技能点。

每篇文章的作者在发布时声明前置知识，比如："读这篇文章，你需要已经掌握 Linear Logic（必须）和 Game Semantics（推荐）。"

然后系统只把这篇文章推给点亮了对应 Tag 的人。就这么简单。

Tag 的组织方式完全由你决定。你可以认为「高等数学」包含一元微分、积分、级数；别人可以有不同的分法。点亮「高等数学」会自动点亮你定义的所有子 Tag。没有人能垄断"什么是什么的前置知识"这个定义权。

你还可以标记自己想学的领域，系统会更积极地给你推荐相关入门内容。

### 2. Fork：比评论更好的协作方式

我们借鉴了 Federated Wiki 的核心理念：**任何人都可以 Fork 任何内容。**

看到一篇编译原理的讲解觉得某个地方可以说得更清楚？不用写评论——直接 Fork 它，改成你觉得更好的版本。你的 Fork 会以双向链接的形式出现在原文下方，按赞数排序。

不同的人 Fork 同一个知识节点，就会涌现出不同的讲法、不同的学习路线。高赞的 Fork 自然演化成社区公认的好内容，有点像 wiki，但更灵活。

你甚至可以把你的修改作为一个 PR 发给原作者，让他合并你的改进。

这一切的底层由 [Pijul](https://pijul.org/) 驱动——一个基于范畴论 patch theory 的版本控制系统。它的 patch 可交换，合并顺序无关，天然适合这种去中心化的协作场景。

### 3. 你的数据，你做主

Fedi-Xanadu 建立在 [AT Protocol](https://atproto.com)（Bluesky 背后的协议）之上，而不是 ActivityPub。原因很现实：ActivityPub 的账号迁移不能带着历史记录走。如果你因为过去发布的内容被绑架在一个实例上，那所谓的"去中心化"就是假的。

在 Fedi-Xanadu 上：

- **你的文章、Fork、所有数据都存在你自己的 PDS**（Personal Data Server），不在平台手里
- Fork 一篇文章 = 把完整内容拉进你自己的 PDS，后续编辑是在你自己的数据上操作
- 平台（AppView）只是一个索引和展示层。它挂了，你的数据零丢失，换一个 AppView 重新索引就行
- 账号迁移带着全部历史记录走，真正的去中心化

## 为什么不是现有的平台？

- **知乎/Medium**：中心化平台，内容分发由平台算法控制，没有前置知识概念
- **Wikipedia**：优秀，但只允许"一个正确版本"，不鼓励多元表达
- **GitHub Wiki**：没有社交分发，没有前置知识匹配
- **Obsidian Publish**：个人笔记发布，没有协作和 Fork
- **Bluesky/Mastodon**：社交平台，不是知识管理工具

Fedi-Xanadu 不是要替代它们，而是填补一个空白：**面向学术和技术交流的、有前置知识感知的、联邦式的知识协作平台。**

## 其他你可能关心的

- **内容格式**：支持 Typst，数学公式书写极其轻松，告别 LaTeX 的痛苦
- **许可证**：所有内容默认 CC-BY-NC-SA 4.0（知识共享-署名-非商业性使用-相同方式共享），你也可以选择更宽松的 CC0 或 CC-BY-SA
- **个人网站融合**（规划中）：你可以只放一个标题和摘要，点击跳转到你的个人博客。计划开发 Hexo、Astro、Hugo、Hakyll 的插件。没有什么比个人网站更能代表互联网的初心了
- **自托管**：用 Rust 写的，单二进制部署，Nix 一键部署。哪怕你只有一个用户，也能在自己的 HomeLab 上跑一个实例
- **开源**：AGPL-3.0
`;var l=d('<div class="content svelte-8kinj7"></div>');function g(a,i){o(i,!1);const t=u.parse(k);p();var n=l();s(n,()=>t,!0),e(a,n),r()}export{g as default};
