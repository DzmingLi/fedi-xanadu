<script lang="ts">
  import { listListings, matchedListings, searchTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { fmtRep, authorName } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { Listing, ListingKind } from '../lib/types';

  const KIND_LABELS: Record<ListingKind, string> = {
    phd: 'PhD',
    masters: 'Masters',
    ra: 'Research Assistant',
    postdoc: 'Postdoc',
    intern: 'Internship',
    faculty: 'Faculty',
    other: 'Other',
  };

  let listings = $state<Listing[]>([]);
  let matched = $state<Listing[]>([]);
  let loading = $state(true);
  let kindFilter = $state<string>('');
  let showMatched = $state(false);
  let isLoggedIn = $derived(!!getAuth());

  async function load() {
    loading = true;
    try {
      listings = await listListings(kindFilter || undefined);
      if (isLoggedIn) {
        try { matched = await matchedListings(); } catch { matched = []; }
      }
    } catch { /* */ }
    loading = false;
  }

  $effect(() => {
    document.title = 'Listings — NightBoat';
    load();
  });

  function fmtDeadline(d: string | null) {
    if (!d) return 'Rolling';
    return new Date(d).toLocaleDateString();
  }
</script>

<div class="listings-page">
  <div class="page-header">
    <h1>Academic Listings</h1>
    {#if isLoggedIn}
      <a href="/new-listing" class="btn-new">Post Listing</a>
    {/if}
  </div>

  <div class="filters">
    <button class:active={kindFilter === ''} onclick={() => { kindFilter = ''; load(); }}>All</button>
    {#each Object.entries(KIND_LABELS) as [k, label]}
      <button class:active={kindFilter === k} onclick={() => { kindFilter = k; load(); }}>{label}</button>
    {/each}
  </div>

  {#if isLoggedIn && matched.length > 0}
    <section class="matched-section">
      <h2>Matched for You</h2>
      <p class="hint">Based on your skill tree — positions where you meet the requirements.</p>
      <div class="listing-list">
        {#each matched as l}
          <a href="/listing?id={encodeURIComponent(l.id)}" class="listing-card matched">
            <div class="card-top">
              <span class="kind-badge">{KIND_LABELS[l.kind] || l.kind}</span>
              <span class="institution">{l.institution}</span>
              {#if l.department}<span class="dept">· {l.department}</span>{/if}
            </div>
            <h3>{l.title}</h3>
            <div class="card-meta">
              <span>@{l.author_handle || l.did.slice(0, 12)}</span>
              {#if l.author_reputation > 0}<span class="rep">{fmtRep(l.author_reputation)}</span>{/if}
              <span>· Deadline: {fmtDeadline(l.deadline)}</span>
              {#if l.location}<span>· {l.location}</span>{/if}
            </div>
            {#if l.required_tags.length > 0}
              <div class="tags">
                {#each l.required_tags as tag}
                  <span class="tag required">{tag}</span>
                {/each}
                {#each l.preferred_tags as tag}
                  <span class="tag preferred">{tag}</span>
                {/each}
              </div>
            {/if}
          </a>
        {/each}
      </div>
    </section>
  {/if}

  <section>
    <h2>{kindFilter ? KIND_LABELS[kindFilter] || kindFilter : 'All Listings'}</h2>
    {#if loading}
      <div class="empty">Loading...</div>
    {:else if listings.length === 0}
      <div class="empty">No open listings</div>
    {:else}
      <div class="listing-list">
        {#each listings as l}
          <a href="/listing?id={encodeURIComponent(l.id)}" class="listing-card">
            <div class="card-top">
              <span class="kind-badge">{KIND_LABELS[l.kind] || l.kind}</span>
              <span class="institution">{l.institution}</span>
              {#if l.department}<span class="dept">· {l.department}</span>{/if}
            </div>
            <h3>{l.title}</h3>
            {#if l.description}
              <p class="desc">{l.description.slice(0, 200)}{l.description.length > 200 ? '...' : ''}</p>
            {/if}
            <div class="card-meta">
              <span>@{l.author_handle || l.did.slice(0, 12)}</span>
              {#if l.author_reputation > 0}<span class="rep">{fmtRep(l.author_reputation)}</span>{/if}
              <span>· Deadline: {fmtDeadline(l.deadline)}</span>
              {#if l.location}<span>· {l.location}</span>{/if}
              {#if l.compensation}<span>· {l.compensation}</span>{/if}
            </div>
            {#if l.required_tags.length > 0 || l.preferred_tags.length > 0}
              <div class="tags">
                {#each l.required_tags as tag}
                  <span class="tag required">{tag}</span>
                {/each}
                {#each l.preferred_tags as tag}
                  <span class="tag preferred">{tag}</span>
                {/each}
              </div>
            {/if}
          </a>
        {/each}
      </div>
    {/if}
  </section>
</div>

<style>
  .listings-page { max-width: 760px; margin: 0 auto; }
  .page-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 1rem; }
  .page-header h1 { font-family: var(--font-serif); font-weight: 400; margin: 0; }
  .btn-new { font-size: 13px; padding: 6px 14px; border: 1px solid var(--accent); border-radius: 3px; color: var(--accent); text-decoration: none; transition: all 0.15s; }
  .btn-new:hover { background: var(--accent); color: white; text-decoration: none; }

  .filters { display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 1.5rem; }
  .filters button { padding: 4px 12px; font-size: 12px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; }
  .filters button.active { background: var(--accent); color: white; border-color: var(--accent); }

  h2 { font-family: var(--font-serif); font-weight: 400; font-size: 1.1rem; margin: 1.5rem 0 0.5rem; }
  .hint { font-size: 13px; color: var(--text-hint); margin: 0 0 12px; }

  .matched-section { border-bottom: 1px solid var(--border); padding-bottom: 1rem; margin-bottom: 1rem; }

  .listing-list { display: flex; flex-direction: column; gap: 8px; }
  .listing-card { display: block; padding: 14px 16px; border: 1px solid var(--border); border-radius: 6px; text-decoration: none; color: var(--text-primary); transition: border-color 0.15s; }
  .listing-card:hover { border-color: var(--accent); text-decoration: none; }
  .listing-card.matched { border-left: 3px solid var(--accent); }

  .card-top { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--text-secondary); margin-bottom: 4px; }
  .kind-badge { font-size: 11px; font-weight: 600; text-transform: uppercase; padding: 2px 6px; border-radius: 3px; background: var(--bg-page); border: 1px solid var(--border); color: var(--text-secondary); }
  .institution { font-weight: 500; }
  .dept { color: var(--text-hint); }

  .listing-card h3 { margin: 0 0 4px; font-size: 15px; font-family: var(--font-serif); font-weight: 400; }
  .desc { margin: 0 0 6px; font-size: 13px; color: var(--text-secondary); line-height: 1.4; }

  .card-meta { font-size: 12px; color: var(--text-hint); display: flex; flex-wrap: wrap; gap: 0 4px; }
  .rep { font-size: 11px; font-weight: 600; color: var(--text-secondary); background: var(--bg-page); border: 1px solid var(--border); border-radius: 3px; padding: 0 3px; }

  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 8px; }
  .tag { font-size: 11px; padding: 2px 6px; border-radius: 3px; }
  .tag.required { background: #fef3c7; color: #92400e; }
  .tag.preferred { background: #e0e7ff; color: #3730a3; }

  .empty { padding: 2rem; text-align: center; color: var(--text-hint); font-size: 14px; }
</style>
