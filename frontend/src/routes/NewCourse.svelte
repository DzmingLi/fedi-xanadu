<script lang="ts">
  import { createCourse } from '../lib/api';
  import { t, getLocale, onLocaleChange } from '../lib/i18n';
  import { getAuth } from '../lib/auth';
  import { toast } from '../lib/components/Toast.svelte';

  let locale = $state(getLocale());
  $effect(() => onLocaleChange(() => { locale = getLocale(); }));

  let { canonicalId = '', canonicalTitle = '' }: { canonicalId?: string; canonicalTitle?: string } = $props();

  let title = $state('');
  let description = $state('');
  let coverUrl = $state('');
  let scheduleType = $state('weekly');
  let term = $state('');
  let year = $state('');
  let submitting = $state(false);

  async function submit() {
    if (!title.trim()) return;
    submitting = true;
    try {
      const course = await createCourse({
        title: title.trim(),
        description: description.trim() || undefined,
        cover_url: coverUrl.trim() || undefined,
        schedule_type: scheduleType,
        term: term.trim() || undefined,
        year: year ? parseInt(year) : undefined,
        canonical_id: canonicalId || undefined,
      });
      window.location.hash = `#/course?id=${encodeURIComponent(course.id)}`;
    } catch (e: any) {
      toast(e.message, 'error');
    }
    submitting = false;
  }
</script>

<h1>{t('courses.create')}</h1>

{#if !getAuth()}
  <p>{t('nav.login')}</p>
{:else}
  <form onsubmit={(e) => { e.preventDefault(); submit(); }}>
    <label>
      {t('courses.title')} *
      <input type="text" bind:value={title} required />
    </label>

    <label>
      {t('courses.description')}
      <textarea bind:value={description} rows="3"></textarea>
    </label>

    <label>
      {t('courses.coverUrl')}
      <input type="url" bind:value={coverUrl} />
    </label>

    <label>
      {t('courses.scheduleType')}
      <select bind:value={scheduleType}>
        <option value="weekly">{t('courses.weekly')}</option>
        <option value="module">{t('courses.module')}</option>
        <option value="custom">{t('courses.custom')}</option>
      </select>
    </label>

    <div class="row">
      <label class="flex1">
        {t('courses.term')}
        <select bind:value={term}>
          <option value="">—</option>
          <option value="Spring">Spring</option>
          <option value="Summer">Summer</option>
          <option value="Fall">Fall</option>
          <option value="Winter">Winter</option>
        </select>
      </label>
      <label class="flex1">
        {t('courses.year')}
        <input type="number" bind:value={year} min="1900" max="2100" placeholder="2026" />
      </label>
    </div>

    {#if canonicalId}
      <p class="hint">{t('courses.newOffering')}: {canonicalTitle}</p>
    {/if}

    <button type="submit" class="btn-primary" disabled={submitting || !title.trim()}>
      {submitting ? t('common.loading') : t('common.create')}
    </button>
  </form>
{/if}

<style>
  h1 { margin-bottom: 16px; }
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 560px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    color: var(--text-secondary);
  }
  input, textarea, select {
    font-size: 14px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
    color: var(--text-primary);
  }
  textarea { resize: vertical; }
  .row { display: flex; gap: 12px; }
  .flex1 { flex: 1; }
  .hint { font-size: 12px; color: var(--text-hint); margin: 0; }
  .btn-primary {
    align-self: flex-start;
    padding: 8px 20px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .btn-primary:disabled { opacity: 0.5; cursor: default; }
  .btn-primary:hover:not(:disabled) { opacity: 0.9; }
</style>
