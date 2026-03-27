<script lang="ts">
  import { listSkillTrees, adoptSkillTree, castVote, getMyVote } from '../lib/api';
  import { getAuth } from '../lib/auth';
  import { t } from '../lib/i18n';
  import { tagName } from '../lib/display';
  import type { SkillTree } from '../lib/types';

  let trees = $state<SkillTree[]>([]);
  let loading = $state(true);
  let filterField = $state('');
  let isLoggedIn = $derived(!!getAuth());
  let filteredTrees = $derived(
    filterField ? trees.filter(tr => tr.tag_id === filterField) : trees
  );

  // Dynamic field list from existing trees
  let availableFields = $derived.by(() => {
    const fieldMap = new Map<string, { id: string; name: string; count: number }>();
    for (const tr of trees) {
      if (!tr.tag_id) continue;
      const existing = fieldMap.get(tr.tag_id);
      if (existing) {
        existing.count++;
      } else {
        const name = tr.tag_names ? tagName(tr.tag_names, tr.tag_name || tr.tag_id, tr.tag_id) : (tr.tag_name || tr.tag_id);
        fieldMap.set(tr.tag_id, { id: tr.tag_id, name, count: 1 });
      }
    }
    return [...fieldMap.values()].sort((a, b) => b.count - a.count);
  });

  $effect(() => {
    listSkillTrees().then(list => { trees = list; loading = false; });
  });

  async function adopt(uri: string) {
    if (!isLoggedIn) return;
    await adoptSkillTree(uri);
    alert(t('skills.adopted'));
  }

  async function vote(uri: string, value: number) {
    if (!isLoggedIn) return;
    await castVote(uri, value);
    trees = await listSkillTrees();
  }
</script>

<div class="header">
  <h1>{t('skills.communityTrees')}</h1>
  {#if isLoggedIn}
    <a href="#/skill-tree/new" class="create-btn">{t('skills.createTree')}</a>
  {/if}
</div>
<p class="subtitle">{t('skills.browseHint')}</p>

  <div class="field-filter">
    <button class="filter-btn" class:active={!filterField} onclick={() => filterField = ''}>{t('home.all')}</button>
    {#each availableFields as f}
      <button class="filter-btn" class:active={filterField === f.id} onclick={() => filterField = f.id}>{f.name} ({f.count})</button>
    {/each}
  </div>

{#if loading}
  <p class="meta">{t('common.loading')}</p>
{:else if trees.length === 0}
  <div class="empty">
    <p>{t('skills.noTrees')}</p>
    {#if isLoggedIn}
      <a href="#/skill-tree/new">{t('skills.createFirst')}</a>
    {/if}
  </div>
{:else}
  <div class="tree-list">
    {#each filteredTrees as tree (tree.at_uri)}
      <div class="tree-card">
        <div class="tree-main">
          <a href="#/skill-tree?uri={encodeURIComponent(tree.at_uri)}" class="tree-title">{tree.title}</a>
          {#if tree.tag_id}
            <span class="field-badge">{tree.tag_names ? tagName(tree.tag_names, tree.tag_name || tree.tag_id, tree.tag_id) : (tree.tag_name || tree.tag_id)}</span>
          {/if}
          {#if tree.forked_from}
            <span class="forked-badge">Fork</span>
          {/if}
          {#if tree.description}
            <p class="tree-desc">{tree.description}</p>
          {/if}
          <div class="tree-meta">
            <span>{tree.author_handle ? `@${tree.author_handle}` : tree.did.slice(0, 20)}</span>
            <span>{tree.edge_count} {t('skills.edgeCount')}</span>
            <span>{tree.adopt_count} {t('skills.adoptCount')}</span>
          </div>
        </div>
        <div class="tree-actions">
          <div class="vote-col">
            <button class="vote-btn" onclick={() => vote(tree.at_uri, 1)} disabled={!isLoggedIn}>▲</button>
            <span class="score">{tree.score ?? 0}</span>
            <button class="vote-btn" onclick={() => vote(tree.at_uri, -1)} disabled={!isLoggedIn}>▼</button>
          </div>
          <button class="adopt-btn" onclick={() => adopt(tree.at_uri)} disabled={!isLoggedIn}>{t('skills.adopt')}</button>
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
