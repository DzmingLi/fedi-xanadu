<script lang="ts">
  import { listSkillTrees, adoptSkillTree, castVote, getMyVote } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { authorName } from '../lib/display';
  import type { SkillTree } from '../lib/types';

  const FIELD_LABELS: Record<string, string> = {
    math: '数学', physics: '物理', cs: '计算机', economics: '经济学',
  };

  let trees = $state<SkillTree[]>([]);
  let loading = $state(true);
  let filterField = $state('');
  let isLoggedIn = $derived(!!getAuth());
  let filteredTrees = $derived(
    filterField ? trees.filter(t => t.field === filterField) : trees
  );
  let availableFields = $derived(
    [...new Set(trees.map(t => t.field).filter((f): f is string => !!f))].sort()
  );

  $effect(() => {
    listSkillTrees().then(t => { trees = t; loading = false; });
  });

  async function adopt(uri: string) {
    if (!isLoggedIn) return;
    await adoptSkillTree(uri);
    alert('已采用该技能树！');
  }

  async function vote(uri: string, value: number) {
    if (!isLoggedIn) return;
    await castVote(uri, value);
    trees = await listSkillTrees();
  }
</script>

<div class="header">
  <h1>Skill Trees</h1>
  {#if isLoggedIn}
    <a href="#/skill-tree/new" class="create-btn">创建技能树</a>
  {/if}
</div>
<p class="subtitle">浏览社区分享的技能树，采用你喜欢的，或 fork 后自定义</p>

  <div class="field-filter">
    <button class="filter-btn" class:active={!filterField} onclick={() => filterField = ''}>全部</button>
    {#each Object.entries(FIELD_LABELS) as [f, label]}
      <button class="filter-btn" class:active={filterField === f} onclick={() => filterField = f}>{label}</button>
    {/each}
  </div>

{#if loading}
  <p class="meta">Loading...</p>
{:else if trees.length === 0}
  <div class="empty">
    <p>还没有技能树</p>
    {#if isLoggedIn}
      <a href="#/skill-tree/new">创建第一棵</a>
    {/if}
  </div>
{:else}
  <div class="tree-list">
    {#each filteredTrees as t (t.at_uri)}
      <div class="tree-card">
        <div class="tree-main">
          <a href="#/skill-tree?uri={encodeURIComponent(t.at_uri)}" class="tree-title">{t.title}</a>
          {#if t.field}
            <span class="field-badge">{FIELD_LABELS[t.field] || t.field}</span>
          {/if}
          {#if t.forked_from}
            <span class="forked-badge">Fork</span>
          {/if}
          {#if t.description}
            <p class="tree-desc">{t.description}</p>
          {/if}
          <div class="tree-meta">
            <span>{t.author_handle ? `@${t.author_handle}` : t.did.slice(0, 20)}</span>
            <span>{t.edge_count} 条关系</span>
            <span>{t.adopt_count} 人采用</span>
          </div>
        </div>
        <div class="tree-actions">
          <div class="vote-col">
            <button class="vote-btn" onclick={() => vote(t.at_uri, 1)} disabled={!isLoggedIn}>▲</button>
            <span class="score">{t.score ?? 0}</span>
            <button class="vote-btn" onclick={() => vote(t.at_uri, -1)} disabled={!isLoggedIn}>▼</button>
          </div>
          <button class="adopt-btn" onclick={() => adopt(t.at_uri)} disabled={!isLoggedIn}>采用</button>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 4px;
  }
  .header h1 { margin: 0; }
  .subtitle {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 0 0 20px;
  }
  .create-btn {
    padding: 6px 16px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border-radius: 4px;
    text-decoration: none;
  }
  .tree-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tree-card {
    display: flex;
    gap: 16px;
    padding: 16px 20px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    transition: border-color 0.15s;
  }
  .tree-card:hover { border-color: var(--border-strong); }
  .tree-main { flex: 1; min-width: 0; }
  .tree-title {
    font-family: var(--font-serif);
    font-size: 1.15rem;
    color: var(--text-primary);
    text-decoration: none;
  }
  .tree-title:hover { color: var(--accent); }
  .forked-badge {
    font-size: 11px;
    background: var(--bg-gray, #f0f0f0);
    color: var(--text-hint);
    padding: 1px 6px;
    border-radius: 3px;
    margin-left: 6px;
  }
  .tree-desc {
    font-size: 14px;
    color: var(--text-secondary);
    margin: 4px 0 0;
    line-height: 1.5;
  }
  .tree-meta {
    display: flex;
    gap: 12px;
    margin-top: 8px;
    font-size: 13px;
    color: var(--text-hint);
  }
  .tree-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .vote-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .vote-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 12px;
    padding: 2px 4px;
    transition: color 0.15s;
  }
  .vote-btn:hover:not(:disabled) { color: var(--accent); }
  .vote-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .score {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .adopt-btn {
    padding: 6px 14px;
    font-size: 13px;
    border: 1px solid var(--accent);
    background: none;
    color: var(--accent);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }
  .adopt-btn:hover:not(:disabled) { background: var(--accent); color: white; }
  .adopt-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .field-filter {
    display: flex;
    gap: 6px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .filter-btn {
    padding: 4px 12px;
    font-size: 13px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--bg-white);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }
  .filter-btn:hover { border-color: var(--accent); color: var(--accent); }
  .filter-btn.active { background: var(--accent); color: white; border-color: var(--accent); }
  .field-badge {
    font-size: 11px;
    background: rgba(95,155,101,0.12);
    color: var(--accent);
    padding: 1px 8px;
    border-radius: 3px;
    margin-left: 6px;
  }
  .empty { color: var(--text-hint); }
</style>
