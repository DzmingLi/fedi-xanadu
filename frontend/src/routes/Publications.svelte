<script lang="ts">
  import { listPublications, createPublication } from '../lib/api';
  import type { PublicationSummary } from '../lib/types';
  import { getAuth } from '../lib/auth.svelte';
  import { t, getLocale } from '../lib/i18n/index.svelte';

  let publications = $state<PublicationSummary[]>([]);
  let loading = $state(true);
  let creating = $state(false);
  let slug = $state('');
  let titleZh = $state('');
  let titleEn = $state('');
  let descZh = $state('');
  let descEn = $state('');
  let coverUrl = $state('');
  let error = $state('');

  $effect(() => { void load(); });

  async function load() {
    loading = true;
    try {
      publications = await listPublications();
    } catch (e) {
      console.error(e);
    }
    loading = false;
  }

  /** Resolve a localized field to current locale, falling back through en/zh/first. */
  function loc(field: Record<string, string> | null | undefined): string {
    if (!field) return '';
    const cur = getLocale();
    return field[cur] || field['en'] || field['zh'] || Object.values(field)[0] || '';
  }

  async function submit() {
    error = '';
    const normalizedSlug = slug.trim();
    if (!/^[a-zA-Z0-9_-]{1,64}$/.test(normalizedSlug)) {
      error = t('publications.slugInvalid');
      return;
    }
    const title_i18n: Record<string, string> = {};
    if (titleZh.trim()) title_i18n['zh'] = titleZh.trim();
    if (titleEn.trim()) title_i18n['en'] = titleEn.trim();
    const description_i18n: Record<string, string> = {};
    if (descZh.trim()) description_i18n['zh'] = descZh.trim();
    if (descEn.trim()) description_i18n['en'] = descEn.trim();
    try {
      await createPublication({
        id: normalizedSlug,
        title_i18n,
        description_i18n,
        cover_url: coverUrl.trim() || null,
      });
      window.location.href = `/publication?id=${encodeURIComponent(normalizedSlug)}`;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="publications-page">
  <header class="pub-header">
    <div>
      <h1>{t('publications.title')}</h1>
      <p class="sub">{t('publications.subtitle')}</p>
    </div>
    {#if getAuth()}
      <button class="primary" onclick={() => { creating = true; }}>
        {t('publications.create')}
      </button>
    {/if}
  </header>

  {#if loading}
    <p class="meta">{t('common.loading')}</p>
  {:else if publications.length === 0}
    <p class="empty">{t('publications.empty')}</p>
  {:else}
    <div class="pub-grid">
      {#each publications as p}
        <a class="pub-card" href="/publication?id={encodeURIComponent(p.id)}">
          {#if p.cover_url}
            <img src={p.cover_url} alt="" class="pub-cover" loading="lazy" />
          {:else}
            <div class="pub-cover placeholder"></div>
          {/if}
          <div class="pub-body">
            <h3 class="pub-title">{loc(p.title_i18n) || p.id}</h3>
            {#if loc(p.description_i18n)}
              <p class="pub-desc">{loc(p.description_i18n)}</p>
            {/if}
            <div class="pub-meta">
              <span>{t('publications.memberCount', p.member_count)}</span>
              <span>{t('publications.contentCount', p.content_count)}</span>
              <span>{t('publications.followerCount', p.follower_count)}</span>
            </div>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>

{#if creating}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={() => { creating = false; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>{t('publications.create')}</h3>
      <label>
        <span>{t('publications.slug')}</span>
        <input bind:value={slug} placeholder="ai-weekly" />
        <small>{t('publications.slugHint')}</small>
      </label>
      <div class="two-col">
        <label>
          <span>{t('newArticle.titleLabel')} (zh)</span>
          <input bind:value={titleZh} placeholder="AI 周刊" />
        </label>
        <label>
          <span>{t('newArticle.titleLabel')} (en)</span>
          <input bind:value={titleEn} placeholder="AI Weekly" />
        </label>
      </div>
      <div class="two-col">
        <label>
          <span>{t('newArticle.descLabel')} (zh)</span>
          <textarea bind:value={descZh} rows="2"></textarea>
        </label>
        <label>
          <span>{t('newArticle.descLabel')} (en)</span>
          <textarea bind:value={descEn} rows="2"></textarea>
        </label>
      </div>
      <label>
        <span>{t('publications.coverUrl')}</span>
        <input bind:value={coverUrl} placeholder="https://..." />
      </label>
      {#if error}<p class="error">{error}</p>{/if}
      <div class="actions">
        <button onclick={() => { creating = false; }}>{t('common.cancel')}</button>
        <button class="primary" onclick={submit}>{t('common.create')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .publications-page { max-width: 1080px; margin: 0 auto; padding: 1.5rem 1rem; }
  .pub-header {
    display: flex; align-items: flex-start; justify-content: space-between; gap: 16px;
    margin-bottom: 24px;
  }
  .pub-header h1 {
    margin: 0; font-family: var(--font-serif); font-weight: 400; font-size: 1.5rem;
  }
  .sub { margin: 4px 0 0; color: var(--text-secondary); font-size: 0.875rem; }
  .primary {
    padding: 6px 14px; font-size: 13px; background: var(--accent); color: white;
    border: none; border-radius: 4px; cursor: pointer;
  }
  .primary:hover { opacity: 0.9; }
  .meta, .empty { text-align: center; color: var(--text-hint); padding: 40px 0; }
  .pub-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 16px; }
  .pub-card {
    display: block; border: 1px solid var(--border); border-radius: 6px; overflow: hidden;
    background: var(--bg-white); color: inherit; text-decoration: none;
    transition: border-color 0.15s, transform 0.15s;
  }
  .pub-card:hover { border-color: var(--accent); transform: translateY(-1px); }
  .pub-cover {
    display: block; width: 100%; aspect-ratio: 3 / 1; object-fit: cover;
    background: var(--bg-hover, #f5f5f5);
  }
  .pub-cover.placeholder {
    background: linear-gradient(135deg, #f3f3ee, #e8e8e0);
  }
  .pub-body { padding: 12px 14px; }
  .pub-title { margin: 0 0 6px; font-size: 1rem; font-weight: 600; }
  .pub-desc {
    margin: 0 0 8px; font-size: 13px; color: var(--text-secondary);
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }
  .pub-meta {
    display: flex; gap: 12px; font-size: 12px; color: var(--text-hint);
  }

  .modal-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; justify-content: center; align-items: center; z-index: 500;
  }
  .modal {
    background: var(--bg-white); border-radius: 6px; padding: 20px;
    width: 600px; max-width: 92vw; max-height: 85vh; overflow-y: auto;
  }
  .modal h3 { margin: 0 0 16px; font-family: var(--font-serif); font-weight: 400; }
  .modal label { display: block; margin-bottom: 12px; }
  .modal label > span { display: block; font-size: 12px; color: var(--text-secondary); margin-bottom: 4px; }
  .modal small { display: block; font-size: 11px; color: var(--text-hint); margin-top: 2px; }
  .modal input, .modal textarea {
    width: 100%; padding: 6px 8px; font-size: 13px;
    border: 1px solid var(--border); border-radius: 3px; background: var(--bg-white);
    color: var(--text-primary); font-family: inherit;
  }
  .modal input:focus, .modal textarea:focus { outline: none; border-color: var(--accent); }
  .two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .error { color: #dc2626; font-size: 13px; margin: 8px 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
  .actions button {
    padding: 6px 14px; font-size: 13px; border: 1px solid var(--border);
    border-radius: 3px; background: none; cursor: pointer;
  }
  .actions button.primary { background: var(--accent); color: white; border-color: var(--accent); }
</style>
