<script lang="ts">
  import { createListing, updateListing, getListing, searchTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import type { ListingKind, Tag } from '../lib/types';

  let { editId = '' }: { editId?: string } = $props();

  const KINDS: { value: ListingKind; label: string }[] = [
    { value: 'phd', label: 'PhD Student' },
    { value: 'masters', label: 'Masters Student' },
    { value: 'ra', label: 'Research Assistant' },
    { value: 'postdoc', label: 'Postdoc' },
    { value: 'intern', label: 'Internship' },
    { value: 'faculty', label: 'Faculty Position' },
    { value: 'other', label: 'Other' },
  ];

  let title = $state('');
  let description = $state('');
  let kind = $state<ListingKind>('phd');
  let institution = $state('');
  let department = $state('');
  let location = $state('');
  let contactEmail = $state('');
  let contactUrl = $state('');
  let compensation = $state('');
  let deadline = $state('');
  let requiredTags = $state<string[]>([]);
  let preferredTags = $state<string[]>([]);

  // Tag search
  let tagQuery = $state('');
  let tagResults = $state<Tag[]>([]);
  let tagTarget = $state<'required' | 'preferred'>('required');
  let tagTimer: ReturnType<typeof setTimeout> | undefined;

  let submitting = $state(false);
  let error = $state('');

  // Load existing listing for editing
  $effect(() => {
    if (editId) {
      getListing(editId).then(l => {
        title = l.title;
        description = l.description;
        kind = l.kind;
        institution = l.institution;
        department = l.department || '';
        location = l.location || '';
        contactEmail = l.contact_email || '';
        contactUrl = l.contact_url || '';
        compensation = l.compensation || '';
        deadline = l.deadline || '';
        requiredTags = l.required_tags;
        preferredTags = l.preferred_tags;
      }).catch(e => { error = e.message; });
    }
    document.title = editId ? 'Edit Listing — NightBoat' : 'New Listing — NightBoat';
  });

  function onTagInput() {
    const q = tagQuery.trim();
    if (!q) { tagResults = []; return; }
    clearTimeout(tagTimer);
    tagTimer = setTimeout(async () => {
      try { tagResults = await searchTags(q); } catch { tagResults = []; }
    }, 200);
  }

  function addTag(tagId: string) {
    if (tagTarget === 'required' && !requiredTags.includes(tagId)) {
      requiredTags = [...requiredTags, tagId];
    } else if (tagTarget === 'preferred' && !preferredTags.includes(tagId)) {
      preferredTags = [...preferredTags, tagId];
    }
    tagQuery = '';
    tagResults = [];
  }

  function removeRequired(tagId: string) { requiredTags = requiredTags.filter(t => t !== tagId); }
  function removePreferred(tagId: string) { preferredTags = preferredTags.filter(t => t !== tagId); }

  async function submit() {
    if (!title.trim() || !institution.trim()) {
      error = 'Title and institution are required';
      return;
    }
    submitting = true;
    error = '';
    try {
      const data = {
        title: title.trim(),
        description: description.trim() || undefined,
        kind,
        institution: institution.trim(),
        department: department.trim() || undefined,
        location: location.trim() || undefined,
        contact_email: contactEmail.trim() || undefined,
        contact_url: contactUrl.trim() || undefined,
        compensation: compensation.trim() || undefined,
        deadline: deadline || undefined,
        required_tags: requiredTags,
        preferred_tags: preferredTags,
      } as any;

      const listing = editId
        ? await updateListing(editId, data)
        : await createListing(data);
      window.location.href = `/listing?id=${encodeURIComponent(listing.id)}`;
    } catch (e: any) {
      error = e.message || 'Failed';
    }
    submitting = false;
  }
</script>

<div class="form-page">
  <h1>{editId ? 'Edit Listing' : 'Post Academic Listing'}</h1>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <label>
    Position Type
    <select bind:value={kind}>
      {#each KINDS as k}
        <option value={k.value}>{k.label}</option>
      {/each}
    </select>
  </label>

  <label>
    Title <span class="req">*</span>
    <input type="text" bind:value={title} placeholder="e.g. PhD position in Computer Vision" />
  </label>

  <label>
    Institution <span class="req">*</span>
    <input type="text" bind:value={institution} placeholder="e.g. Peking University" />
  </label>

  <label>
    Department
    <input type="text" bind:value={department} placeholder="e.g. School of Computer Science" />
  </label>

  <label>
    Location
    <input type="text" bind:value={location} placeholder="e.g. Beijing, China" />
  </label>

  <label>
    Description
    <textarea bind:value={description} rows="6" placeholder="Position details, research direction, requirements..."></textarea>
  </label>

  <label>
    Compensation / Funding
    <input type="text" bind:value={compensation} placeholder="e.g. ¥3,000/month stipend + tuition waiver" />
  </label>

  <label>
    Application Deadline
    <input type="date" bind:value={deadline} />
    <span class="hint">Leave empty for rolling applications</span>
  </label>

  <label>
    Contact Email
    <input type="email" bind:value={contactEmail} placeholder="professor@university.edu" />
  </label>

  <label>
    Application Link
    <input type="url" bind:value={contactUrl} placeholder="https://..." />
  </label>

  <fieldset>
    <legend>Skill Requirements</legend>
    <div class="tag-input-row">
      <select bind:value={tagTarget}>
        <option value="required">Required</option>
        <option value="preferred">Preferred</option>
      </select>
      <input type="text" bind:value={tagQuery} oninput={onTagInput} placeholder="Search tags..." />
    </div>
    {#if tagResults.length > 0}
      <div class="tag-dropdown">
        {#each tagResults as tag}
          <button type="button" onclick={() => addTag(tag.id)}>{tag.name} <span class="tag-id">({tag.id})</span></button>
        {/each}
      </div>
    {/if}

    {#if requiredTags.length > 0}
      <div class="tag-group">
        <span class="tag-label">Required:</span>
        {#each requiredTags as tag}
          <span class="tag required">{tag} <button type="button" onclick={() => removeRequired(tag)}>x</button></span>
        {/each}
      </div>
    {/if}
    {#if preferredTags.length > 0}
      <div class="tag-group">
        <span class="tag-label">Preferred:</span>
        {#each preferredTags as tag}
          <span class="tag preferred">{tag} <button type="button" onclick={() => removePreferred(tag)}>x</button></span>
        {/each}
      </div>
    {/if}
  </fieldset>

  <div class="actions">
    <button class="btn-submit" onclick={submit} disabled={submitting || !title.trim() || !institution.trim()}>
      {submitting ? 'Submitting...' : editId ? 'Update' : 'Post Listing'}
    </button>
  </div>
</div>

<style>
  .form-page { max-width: 600px; margin: 0 auto; }
  h1 { font-family: var(--font-serif); font-weight: 400; margin: 0 0 1.5rem; }
  .error { background: #fef2f2; color: #dc2626; padding: 8px 12px; border-radius: 4px; font-size: 13px; margin-bottom: 12px; }

  label { display: block; font-size: 13px; font-weight: 500; color: var(--text-secondary); margin-bottom: 14px; }
  .req { color: #dc2626; }
  input, select, textarea { display: block; width: 100%; margin-top: 4px; padding: 8px 10px; font-size: 14px; border: 1px solid var(--border); border-radius: 4px; font-family: var(--font-sans); background: var(--bg-white); }
  textarea { resize: vertical; }
  input:focus, select:focus, textarea:focus { outline: none; border-color: var(--accent); }
  .hint { display: block; font-size: 12px; color: var(--text-hint); margin-top: 2px; font-weight: 400; }

  fieldset { border: 1px solid var(--border); border-radius: 6px; padding: 14px; margin-bottom: 14px; }
  legend { font-size: 13px; font-weight: 600; color: var(--text-secondary); padding: 0 6px; }

  .tag-input-row { display: flex; gap: 8px; }
  .tag-input-row select { width: 120px; flex-shrink: 0; }
  .tag-input-row input { flex: 1; }

  .tag-dropdown { display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: 4px; margin-top: 4px; max-height: 150px; overflow-y: auto; }
  .tag-dropdown button { text-align: left; padding: 6px 10px; border: none; border-bottom: 1px solid var(--border); background: none; cursor: pointer; font-size: 13px; }
  .tag-dropdown button:hover { background: var(--bg-hover); }
  .tag-dropdown button:last-child { border-bottom: none; }
  .tag-id { color: var(--text-hint); font-size: 11px; }

  .tag-group { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; margin-top: 8px; }
  .tag-label { font-size: 12px; color: var(--text-hint); font-weight: 600; }
  .tag { font-size: 12px; padding: 3px 6px; border-radius: 3px; display: inline-flex; align-items: center; gap: 4px; }
  .tag.required { background: #fef3c7; color: #92400e; }
  .tag.preferred { background: #e0e7ff; color: #3730a3; }
  .tag button { background: none; border: none; cursor: pointer; font-size: 11px; opacity: 0.6; padding: 0; }
  .tag button:hover { opacity: 1; }

  .actions { margin-top: 1.5rem; }
  .btn-submit { padding: 8px 20px; font-size: 14px; border: none; border-radius: 4px; background: var(--accent); color: white; cursor: pointer; }
  .btn-submit:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
