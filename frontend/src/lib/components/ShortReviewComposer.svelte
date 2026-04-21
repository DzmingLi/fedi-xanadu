<script lang="ts">
  import { t } from '../i18n/index.svelte';

  let { onSubmit, initialBody = '', placeholder = '' }: {
    onSubmit: (body: string) => Promise<void>;
    initialBody?: string;
    placeholder?: string;
  } = $props();

  let body = $state(initialBody);
  let submitting = $state(false);
  let error = $state('');

  const MAX_LEN = 500;

  async function submit() {
    if (!body.trim()) return;
    submitting = true;
    error = '';
    try {
      await onSubmit(body.trim());
      body = '';
    } catch (e: any) {
      error = e?.message || 'Error';
    } finally {
      submitting = false;
    }
  }
</script>

<div class="composer">
  <textarea
    bind:value={body}
    rows="3"
    maxlength={MAX_LEN}
    placeholder={placeholder || t('books.shortReviewPlaceholder')}
    class="body-input"
  ></textarea>
  <div class="composer-footer">
    <span class="char-count" class:warn={body.length > MAX_LEN * 0.9}>{body.length}/{MAX_LEN}</span>
    {#if error}<span class="error">{error}</span>{/if}
    <button
      class="submit-btn"
      disabled={submitting || !body.trim()}
      onclick={submit}
    >{submitting ? t('books.shortReviewSubmitting') : t('books.shortReviewSubmit')}</button>
  </div>
</div>

<style>
  .composer { display: flex; flex-direction: column; gap: 8px; }
  .body-input {
    width: 100%;
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 6px;
    padding: 8px 10px;
    font-size: 0.9rem;
    resize: vertical;
    font-family: inherit;
    box-sizing: border-box;
  }
  .body-input:focus { outline: none; border-color: var(--accent, #6366f1); }
  .composer-footer { display: flex; align-items: center; gap: 10px; }
  .char-count { font-size: 0.8rem; color: var(--text-muted, #6b7280); }
  .char-count.warn { color: #f59e0b; }
  .error { font-size: 0.85rem; color: #dc2626; flex: 1; }
  .submit-btn {
    margin-left: auto;
    padding: 6px 16px;
    background: var(--accent, #6366f1);
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
