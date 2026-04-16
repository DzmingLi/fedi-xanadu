<script lang="ts">
  import { createEvent, updateEvent, getEventById, searchTags } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { t } from '../lib/i18n/index.svelte';
  import type { EventKind, Tag } from '../lib/types';

  let { editId = '' }: { editId?: string } = $props();

  const KINDS: { value: EventKind; label: string }[] = [
    { value: 'conference', label: 'events.kind.conference' },
    { value: 'workshop', label: 'events.kind.workshop' },
    { value: 'seminar', label: 'events.kind.seminar' },
    { value: 'meetup', label: 'events.kind.meetup' },
    { value: 'hackathon', label: 'events.kind.hackathon' },
  ];

  let title = $state('');
  let description = $state('');
  let kind = $state<EventKind>('meetup');
  let organizer = $state('');
  let location = $state('');
  let onlineUrl = $state('');
  let startTime = $state('');
  let endTime = $state('');
  let maxAttendees = $state('');
  let contactEmail = $state('');
  let contactUrl = $state('');
  let teaches = $state<string[]>([]);
  let prereqs = $state<string[]>([]);

  // Tag search
  let tagQuery = $state('');
  let tagResults = $state<Tag[]>([]);
  let tagTarget = $state<'teaches' | 'prereqs'>('teaches');
  let tagTimer: ReturnType<typeof setTimeout> | undefined;

  let submitting = $state(false);
  let error = $state('');

  // Load existing event for editing
  $effect(() => {
    if (editId) {
      getEventById(editId).then(ev => {
        title = ev.title;
        description = ev.description;
        kind = ev.kind;
        organizer = ev.organizer;
        location = ev.location || '';
        onlineUrl = ev.online_url || '';
        startTime = ev.start_time ? ev.start_time.slice(0, 16) : '';
        endTime = ev.end_time ? ev.end_time.slice(0, 16) : '';
        maxAttendees = ev.max_attendees ? String(ev.max_attendees) : '';
        contactEmail = ev.contact_email || '';
        contactUrl = ev.contact_url || '';
        teaches = ev.teaches;
        prereqs = ev.prereqs;
      }).catch(e => { error = e.message; });
    }
    document.title = editId ? 'Edit Event — NightBoat' : 'New Event — NightBoat';
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
    if (tagTarget === 'teaches' && !teaches.includes(tagId)) {
      teaches = [...teaches, tagId];
    } else if (tagTarget === 'prereqs' && !prereqs.includes(tagId)) {
      prereqs = [...prereqs, tagId];
    }
    tagQuery = '';
    tagResults = [];
  }

  function removeTeach(tagId: string) { teaches = teaches.filter(t => t !== tagId); }
  function removePrereq(tagId: string) { prereqs = prereqs.filter(t => t !== tagId); }

  async function submit() {
    if (!title.trim() || !organizer.trim() || !startTime) {
      error = 'Title, organizer, and start time are required';
      return;
    }
    submitting = true;
    error = '';
    try {
      const data = {
        title: title.trim(),
        description: description.trim(),
        kind,
        organizer: organizer.trim(),
        location: location.trim() || null,
        online_url: onlineUrl.trim() || null,
        start_time: new Date(startTime).toISOString(),
        end_time: endTime ? new Date(endTime).toISOString() : null,
        max_attendees: maxAttendees ? parseInt(maxAttendees) : null,
        contact_email: contactEmail.trim() || null,
        contact_url: contactUrl.trim() || null,
        teaches,
        prereqs,
      };

      const ev = editId
        ? await updateEvent(editId, data)
        : await createEvent(data);
      window.location.href = `/event?id=${encodeURIComponent(ev.id)}`;
    } catch (e: any) {
      error = e.message || 'Failed';
    }
    submitting = false;
  }
</script>

<div class="form-page">
  <h1>{editId ? 'Edit Event' : t('events.create')}</h1>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <label>
    {t('events.kindLabel')}
    <select bind:value={kind}>
      {#each KINDS as k}
        <option value={k.value}>{t(k.label)}</option>
      {/each}
    </select>
  </label>

  <label>
    {t('events.titleLabel')} <span class="req">*</span>
    <input type="text" bind:value={title} placeholder={t('events.titleLabel')} />
  </label>

  <label>
    {t('events.organizerLabel')} <span class="req">*</span>
    <input type="text" bind:value={organizer} placeholder={t('events.organizerLabel')} />
  </label>

  <label>
    {t('events.locationLabel')}
    <input type="text" bind:value={location} placeholder={t('events.locationLabel')} />
  </label>

  <label>
    {t('events.onlineUrlLabel')}
    <input type="url" bind:value={onlineUrl} placeholder="https://..." />
  </label>

  <label>
    {t('events.description')}
    <textarea bind:value={description} rows="6" placeholder={t('events.descPlaceholder')}></textarea>
  </label>

  <label>
    {t('events.startTime')} <span class="req">*</span>
    <input type="datetime-local" bind:value={startTime} />
  </label>

  <label>
    {t('events.endTime')}
    <input type="datetime-local" bind:value={endTime} />
  </label>

  <label>
    {t('events.maxAttendeesLabel')}
    <input type="number" bind:value={maxAttendees} min="1" placeholder="e.g. 100" />
  </label>

  <label>
    {t('events.contactEmail')}
    <input type="email" bind:value={contactEmail} placeholder="organizer@example.com" />
  </label>

  <label>
    {t('events.contactUrl')}
    <input type="url" bind:value={contactUrl} placeholder="https://..." />
  </label>

  <fieldset>
    <legend>{t('events.teaches')} / {t('events.prereqs')}</legend>
    <div class="tag-input-row">
      <select bind:value={tagTarget}>
        <option value="teaches">{t('events.teaches')}</option>
        <option value="prereqs">{t('events.prereqs')}</option>
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

    {#if teaches.length > 0}
      <div class="tag-group">
        <span class="tag-label">{t('events.teaches')}:</span>
        {#each teaches as tag}
          <span class="tag teaches">{tag} <button type="button" onclick={() => removeTeach(tag)}>x</button></span>
        {/each}
      </div>
    {/if}
    {#if prereqs.length > 0}
      <div class="tag-group">
        <span class="tag-label">{t('events.prereqs')}:</span>
        {#each prereqs as tag}
          <span class="tag prereq">{tag} <button type="button" onclick={() => removePrereq(tag)}>x</button></span>
        {/each}
      </div>
    {/if}
  </fieldset>

  <div class="actions">
    <button class="btn-submit" onclick={submit} disabled={submitting || !title.trim() || !organizer.trim() || !startTime}>
      {submitting ? t('events.creating') : editId ? 'Update' : t('events.create')}
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
  .tag.teaches { background: #e0e7ff; color: #3730a3; }
  .tag.prereq { background: #fef3c7; color: #92400e; }
  .tag button { background: none; border: none; cursor: pointer; font-size: 11px; opacity: 0.6; padding: 0; }
  .tag button:hover { opacity: 1; }

  .actions { margin-top: 1.5rem; }
  .btn-submit { padding: 8px 20px; font-size: 14px; border: none; border-radius: 4px; background: var(--accent); color: white; cursor: pointer; }
  .btn-submit:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
