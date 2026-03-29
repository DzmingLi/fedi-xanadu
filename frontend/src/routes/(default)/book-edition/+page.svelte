<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { addBookEdition } from '$lib/api';
  import { getAuth } from '$lib/auth.svelte';
  import { t } from '$lib/i18n/index.svelte';

  let bookId = $derived($page.url.searchParams.get('bookId') ?? '');

  let title = $state('');
  let lang = $state('zh');
  let isbn = $state('');
  let publisher = $state('');
  let year = $state('');
  let coverUrl = $state('');
  let translatorsText = $state('');
  let purchaseLinks = $state<{ label: string; url: string }[]>([]);
  let newLinkLabel = $state('');
  let newLinkUrl = $state('');
  let error = $state('');
  let submitting = $state(false);

  function addLink() {
    if (!newLinkLabel.trim() || !newLinkUrl.trim()) return;
    purchaseLinks = [...purchaseLinks, { label: newLinkLabel.trim(), url: newLinkUrl.trim() }];
    newLinkLabel = '';
    newLinkUrl = '';
  }

  function removeLink(idx: number) {
    purchaseLinks = purchaseLinks.filter((_, i) => i !== idx);
  }

  async function submit() {
    if (!getAuth()) { error = t('auth.submit'); return; }
    if (!title.trim()) { error = t('bookEdition.errTitle'); return; }

    submitting = true;
    error = '';
    try {
      const translators = translatorsText.trim()
        ? translatorsText.split(/[,，]/).map(s => s.trim()).filter(Boolean)
        : [];

      await addBookEdition(bookId, {
        title: title.trim(),
        lang,
        isbn: isbn.trim() || undefined,
        publisher: publisher.trim() || undefined,
        year: year.trim() || undefined,
        translators: translators.length > 0 ? translators : undefined,
        purchase_links: purchaseLinks.length > 0 ? purchaseLinks : undefined,
        cover_url: coverUrl.trim() || undefined,
      });

      goto(`/book?id=${encodeURIComponent(bookId)}`);
    } catch (e: any) {
      error = e.message || t('bookEdition.errCreate');
    }
    submitting = false;
  }
</script>

<h1>{t('bookEdition.title')}</h1>

{#if error}
  <p class="error">{error}</p>
{/if}

<div class="form">
  <label>
    {t('bookEdition.editionTitle')}
    <input type="text" bind:value={title} placeholder={t('bookEdition.editionTitlePlaceholder')} />
  </label>

  <label>
    {t('bookEdition.lang')}
    <select bind:value={lang}>
      <option value="zh">中文</option>
      <option value="en">English</option>
      <option value="fr">Français</option>
      <option value="ja">日本語</option>
      <option value="de">Deutsch</option>
      <option value="es">Español</option>
    </select>
  </label>

  <label>
    ISBN
    <input type="text" bind:value={isbn} placeholder="978-..." />
  </label>

  <label>
    {t('bookEdition.publisher')}
    <input type="text" bind:value={publisher} />
  </label>

  <label>
    {t('bookEdition.year')}
    <input type="text" bind:value={year} placeholder="2024" />
  </label>

  <label>
    {t('bookEdition.translators')}
    <input type="text" bind:value={translatorsText} placeholder={t('bookEdition.translatorsPlaceholder')} />
  </label>

  <label>
    {t('bookEdition.coverUrl')}
    <input type="text" bind:value={coverUrl} placeholder="https://..." />
  </label>

  {#if coverUrl.trim()}
    <div class="cover-preview">
      <img src={coverUrl} alt="cover preview" />
    </div>
  {/if}

  <div class="purchase-section">
    <h3>{t('bookEdition.purchaseLinks')}</h3>
    {#each purchaseLinks as link, i}
      <div class="link-row">
        <span class="link-label">{link.label}</span>
        <a href={link.url} target="_blank" rel="noopener" class="link-url">{link.url}</a>
        <button class="remove-btn" onclick={() => removeLink(i)}>×</button>
      </div>
    {/each}
    <div class="add-link-row">
      <input type="text" bind:value={newLinkLabel} placeholder={t('bookEdition.linkLabel')} />
      <input type="text" bind:value={newLinkUrl} placeholder={t('bookEdition.linkUrl')} />
      <button class="add-link-btn" onclick={addLink}>{t('common.add')}</button>
    </div>
  </div>

  <div class="form-actions">
    <button class="submit-btn" onclick={submit} disabled={submitting}>
      {submitting ? t('bookEdition.submitting') : t('bookEdition.submit')}
    </button>
    <a href="/book?id={encodeURIComponent(bookId)}" class="cancel-link">{t('books.cancel')}</a>
  </div>
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 640px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
    color: var(--text-secondary);
  }
  input, select {
    font-family: var(--font-sans);
    font-size: 14px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-white);
  }
  .cover-preview {
    width: 120px;
  }
  .cover-preview img {
    width: 100%;
    border-radius: 4px;
    border: 1px solid var(--border);
  }
  .purchase-section h3 {
    font-family: var(--font-serif);
    font-weight: 400;
    margin: 8px 0 4px;
  }
  .link-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    padding: 6px 0;
  }
  .link-label {
    font-weight: 500;
  }
  .link-url {
    color: var(--accent);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 300px;
  }
  .remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-hint);
    font-size: 16px;
    padding: 0 4px;
  }
  .add-link-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .add-link-row input {
    flex: 1;
  }
  .add-link-btn {
    padding: 8px 16px;
    font-size: 13px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }
  .form-actions {
    margin-top: 16px;
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .submit-btn {
    padding: 10px 24px;
    font-size: 14px;
    background: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .cancel-link {
    font-size: 14px;
    color: var(--text-secondary);
  }
  .error {
    color: var(--error, #c33);
    font-size: 14px;
  }
</style>
