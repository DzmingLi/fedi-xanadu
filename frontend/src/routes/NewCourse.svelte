<script lang="ts">
  import { createCourse } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import { navigate } from '../lib/router';

  let title = $state('');
  let code = $state('');
  let description = $state('');
  let institution = $state('');
  let department = $state('');
  let semester = $state('');
  let lang = $state('zh');
  let sourceUrl = $state('');
  let sourceAttribution = $state('');
  let submitting = $state(false);
  let error = $state('');

  async function submit() {
    if (!title.trim()) return;
    submitting = true;
    error = '';
    try {
      const course = await createCourse({
        title: title.trim(),
        code: code.trim() || undefined,
        description: description.trim() || undefined,
        institution: institution.trim() || undefined,
        department: department.trim() || undefined,
        semester: semester.trim() || undefined,
        lang,
        source_url: sourceUrl.trim() || undefined,
        source_attribution: sourceAttribution.trim() || undefined,
      });
      navigate(`/course?id=${encodeURIComponent(course.id)}`);
    } catch (e: any) {
      error = e.message;
    }
    submitting = false;
  }
</script>

{#if !getAuth()}
  <p class="meta">Please log in to create a course.</p>
{:else}
  <div class="new-course">
    <h1>Create Course</h1>

    <div class="form-field">
      <label>Title *</label>
      <input type="text" bind:value={title} placeholder="Introduction to Algorithms" />
    </div>

    <div class="form-row">
      <div class="form-field">
        <label>Course Code</label>
        <input type="text" bind:value={code} placeholder="CS229, 6.006, 18.06" />
      </div>
      <div class="form-field">
        <label>Semester</label>
        <input type="text" bind:value={semester} placeholder="Fall 2025" />
      </div>
      <div class="form-field">
        <label>Language</label>
        <select bind:value={lang}>
          <option value="zh">Chinese</option>
          <option value="en">English</option>
          <option value="ja">Japanese</option>
        </select>
      </div>
    </div>

    <div class="form-row">
      <div class="form-field">
        <label>Institution</label>
        <input type="text" bind:value={institution} placeholder="MIT, Stanford, HUST" />
      </div>
      <div class="form-field">
        <label>Department</label>
        <input type="text" bind:value={department} placeholder="Computer Science" />
      </div>
    </div>

    <div class="form-field">
      <label>Description</label>
      <textarea bind:value={description} rows="3" placeholder="Brief course description..."></textarea>
    </div>

    <details class="import-section">
      <summary>Importing from external source?</summary>
      <div class="form-field">
        <label>Source URL</label>
        <input type="url" bind:value={sourceUrl} placeholder="https://ocw.mit.edu/courses/..." />
      </div>
      <div class="form-field">
        <label>Attribution</label>
        <input type="text" bind:value={sourceAttribution} placeholder="MIT OpenCourseWare, CC BY-NC-SA 4.0" />
      </div>
    </details>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="form-actions">
      <button class="submit-btn" onclick={submit} disabled={submitting || !title.trim()}>
        {submitting ? 'Creating...' : 'Create Course'}
      </button>
    </div>
  </div>
{/if}

<style>
  .new-course { max-width: 640px; }
  h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.6rem; margin: 0 0 24px; }
  .form-field { margin-bottom: 16px; }
  .form-field label { display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px; color: var(--text-secondary); }
  .form-field input, .form-field textarea, .form-field select {
    width: 100%; padding: 8px 12px; font-size: 14px; border: 1px solid var(--border);
    border-radius: 4px; font-family: var(--font-sans); background: var(--bg-white); color: var(--text-primary);
  }
  .form-field textarea { resize: vertical; }
  .form-row { display: flex; gap: 12px; }
  .form-row .form-field { flex: 1; }
  .import-section { margin: 16px 0; padding: 12px; border: 1px dashed var(--border); border-radius: 4px; }
  .import-section summary { font-size: 13px; color: var(--text-hint); cursor: pointer; }
  .import-section summary:hover { color: var(--accent); }
  .import-section .form-field { margin-top: 12px; }
  .error { font-size: 13px; color: #dc2626; }
  .form-actions { margin-top: 24px; }
  .submit-btn {
    padding: 8px 24px; font-size: 14px; background: var(--accent); color: white;
    border: none; border-radius: 4px; cursor: pointer; transition: opacity 0.15s;
  }
  .submit-btn:hover { opacity: 0.9; }
  .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  @media (max-width: 600px) {
    .form-row { flex-direction: column; gap: 0; }
  }
</style>
