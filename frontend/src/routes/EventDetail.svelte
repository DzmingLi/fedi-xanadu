<script lang="ts">
  import { getEventById, listEventRsvps, rsvpEvent, cancelRsvp, cancelEvent, uncancelEvent, deleteEvent } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { fmtRep } from '../lib/display';
  import { t } from '../lib/i18n/index.svelte';
  import type { NbEvent, EventRsvp, EventKind } from '../lib/types';

  const KIND_KEYS: Record<EventKind, string> = {
    conference: 'events.kind.conference',
    workshop: 'events.kind.workshop',
    seminar: 'events.kind.seminar',
    meetup: 'events.kind.meetup',
    hackathon: 'events.kind.hackathon',
  };

  const KIND_COLORS: Record<EventKind, string> = {
    conference: '#7c3aed',
    workshop: '#059669',
    seminar: '#2563eb',
    meetup: '#d97706',
    hackathon: '#dc2626',
  };

  let { id }: { id: string } = $props();
  let event = $state<NbEvent | null>(null);
  let rsvps = $state<EventRsvp[]>([]);
  let error = $state('');
  let myRsvpStatus = $state<string | null>(null);

  let isOwner = $derived(!!getAuth() && event?.did === getAuth()?.did);
  let isLoggedIn = $derived(!!getAuth());
  let myDid = $derived(getAuth()?.did || '');

  $effect(() => {
    if (!id) return;
    getEventById(id).then(ev => {
      event = ev;
      document.title = `${ev.title} — NightBoat`;
    }).catch(e => { error = e.message; });

    listEventRsvps(id).then(r => {
      rsvps = r;
      const me = getAuth()?.did;
      if (me) {
        const myR = r.find(rv => rv.did === me);
        myRsvpStatus = myR ? myR.status : null;
      }
    }).catch(() => {});
  });

  function fmtDateTime(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' }) +
      ' ' + d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }

  async function doRsvp(status: 'going' | 'interested') {
    if (!event) return;
    try {
      await rsvpEvent(event.id, status);
      myRsvpStatus = status;
      rsvps = await listEventRsvps(event.id);
      event = { ...event, rsvp_count: rsvps.length };
    } catch {}
  }

  async function doCancelRsvp() {
    if (!event) return;
    try {
      await cancelRsvp(event.id);
      myRsvpStatus = null;
      rsvps = await listEventRsvps(event.id);
      event = { ...event, rsvp_count: rsvps.length };
    } catch {}
  }

  async function doCancel() {
    if (!event) return;
    await cancelEvent(event.id);
    event = { ...event, is_cancelled: true };
  }

  async function doUncancel() {
    if (!event) return;
    await uncancelEvent(event.id);
    event = { ...event, is_cancelled: false };
  }

  async function doDelete() {
    if (!event || !confirm(t('events.deleteConfirm'))) return;
    await deleteEvent(event.id);
    window.location.href = '/events';
  }
</script>

{#if error}
  <div class="error">{error}</div>
{:else if !event}
  <p>Loading...</p>
{:else}
  <article class="event-detail">
    <div class="header">
      <span class="kind-badge" style="background: {KIND_COLORS[event.kind]}20; color: {KIND_COLORS[event.kind]}; border-color: {KIND_COLORS[event.kind]}40">{t(KIND_KEYS[event.kind])}</span>
      {#if event.is_cancelled}<span class="cancelled-badge">{t('events.cancelled')}</span>{/if}
      {#if event.max_attendees && event.rsvp_count >= event.max_attendees}<span class="full-badge">{t('events.full')}</span>{/if}
    </div>

    <h1>{event.title}</h1>

    <div class="meta-row">
      <a href="/profile?did={encodeURIComponent(event.did)}" class="author">
        @{event.author_handle || event.did.slice(0, 16)}
      </a>
      {#if event.author_reputation > 0}
        <span class="rep">{fmtRep(event.author_reputation)}</span>
      {/if}
    </div>

    <div class="info-grid">
      <div class="info-item">
        <strong>{t('events.organizer')}</strong>
        <span>{event.organizer}</span>
      </div>
      <div class="info-item">
        <strong>{t('events.location')}</strong>
        <span>{event.location || t('events.online')}</span>
      </div>
      <div class="info-item">
        <strong>{t('events.time')}</strong>
        <span>
          {fmtDateTime(event.start_time)}
          {#if event.end_time} — {fmtDateTime(event.end_time)}{/if}
        </span>
      </div>
      {#if event.online_url}
        <div class="info-item">
          <strong>{t('events.online')}</strong>
          <a href={event.online_url} target="_blank" rel="noopener">{event.online_url}</a>
        </div>
      {/if}
      {#if event.max_attendees}
        <div class="info-item">
          <strong>{t('events.maxAttendees')}</strong>
          <span>{event.rsvp_count} / {event.max_attendees}</span>
        </div>
      {/if}
      {#if event.contact_email}
        <div class="info-item">
          <strong>{t('events.contact')}</strong>
          <a href="mailto:{event.contact_email}">{event.contact_email}</a>
        </div>
      {/if}
      {#if event.contact_url}
        <div class="info-item">
          <strong>{t('events.contact')}</strong>
          <a href={event.contact_url} target="_blank" rel="noopener">{event.contact_url}</a>
        </div>
      {/if}
    </div>

    {#if event.teaches.length > 0 || event.prereqs.length > 0}
      <div class="skills-section">
        {#if event.teaches.length > 0}
          <div class="skill-group">
            <h3>{t('events.teaches')}</h3>
            <div class="tags">
              {#each event.teaches as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag teaches">{tag}</a>
              {/each}
            </div>
          </div>
        {/if}
        {#if event.prereqs.length > 0}
          <div class="skill-group">
            <h3>{t('events.prereqs')}</h3>
            <div class="tags">
              {#each event.prereqs as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag prereq">{tag}</a>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if event.description}
      <div class="description">{event.description}</div>
    {/if}

    <!-- RSVP actions for logged-in non-owner users -->
    {#if isLoggedIn && !isOwner && !event.is_cancelled}
      <div class="rsvp-actions">
        {#if myRsvpStatus}
          <span class="rsvp-status">
            {myRsvpStatus === 'going' ? t('events.going') : t('events.interested')}
          </span>
          <button class="btn" onclick={doCancelRsvp}>{t('events.cancelRsvp')}</button>
        {:else}
          <button class="btn btn-primary" onclick={() => doRsvp('going')}>{t('events.going')}</button>
          <button class="btn" onclick={() => doRsvp('interested')}>{t('events.interested')}</button>
        {/if}
      </div>
    {/if}

    <!-- Attendees -->
    {#if rsvps.length > 0}
      <div class="attendees-section">
        <h3>{t('events.attendees')} ({rsvps.length})</h3>
        <div class="attendee-list">
          {#each rsvps as r}
            <a href="/profile?did={encodeURIComponent(r.did)}" class="attendee">
              <span class="attendee-name">{r.display_name || r.handle || r.did.slice(0, 16)}</span>
              <span class="attendee-status">{r.status === 'going' ? t('events.going') : t('events.interested')}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}

    <div class="post-date">Posted {new Date(event.created_at).toLocaleDateString()}</div>

    {#if isOwner}
      <div class="owner-actions">
        <a href="/new-event?edit={encodeURIComponent(event.id)}" class="btn">Edit</a>
        {#if event.is_cancelled}
          <button class="btn" onclick={doUncancel}>Uncancel</button>
        {:else}
          <button class="btn" onclick={doCancel}>{t('events.cancelled')}</button>
        {/if}
        <button class="btn btn-danger" onclick={doDelete}>Delete</button>
      </div>
    {/if}
  </article>
{/if}

<style>
  .event-detail { max-width: 700px; }
  .error { background: #fef2f2; color: #dc2626; padding: 12px; border-radius: 4px; }

  .header { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; }
  .kind-badge { font-size: 12px; font-weight: 600; text-transform: uppercase; padding: 3px 8px; border-radius: 3px; border: 1px solid; }
  .cancelled-badge { font-size: 12px; font-weight: 600; padding: 3px 8px; border-radius: 3px; background: #fef2f2; color: #dc2626; }
  .full-badge { font-size: 12px; font-weight: 600; padding: 3px 8px; border-radius: 3px; background: #fef3c7; color: #92400e; }

  h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.5rem; margin: 0 0 8px; }

  .meta-row { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: 14px; color: var(--text-secondary); margin-bottom: 1.5rem; }
  .author { color: var(--accent); text-decoration: none; }
  .author:hover { text-decoration: underline; }
  .rep { font-size: 11px; font-weight: 600; color: var(--text-secondary); background: var(--bg-page); border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; }

  .info-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 12px; margin-bottom: 1.5rem; padding: 14px; background: var(--bg-page); border-radius: 6px; border: 1px solid var(--border); }
  .info-item { font-size: 13px; }
  .info-item strong { display: block; font-size: 11px; text-transform: uppercase; color: var(--text-hint); margin-bottom: 2px; }
  .info-item a { color: var(--accent); word-break: break-all; }

  .skills-section { margin-bottom: 1.5rem; }
  .skill-group { margin-bottom: 12px; }
  .skill-group h3 { font-size: 13px; font-weight: 600; margin: 0 0 6px; color: var(--text-secondary); }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; }
  .tag { font-size: 12px; padding: 3px 8px; border-radius: 3px; text-decoration: none; transition: opacity 0.15s; }
  .tag:hover { opacity: 0.8; text-decoration: none; }
  .tag.teaches { background: #e0e7ff; color: #3730a3; }
  .tag.prereq { background: #fef3c7; color: #92400e; }

  .description { font-size: 15px; line-height: 1.7; white-space: pre-wrap; margin-bottom: 1.5rem; }

  .rsvp-actions { display: flex; gap: 8px; align-items: center; padding: 12px 0; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; }
  .rsvp-status { font-size: 13px; font-weight: 600; color: var(--accent); padding: 4px 10px; background: var(--accent)10; border-radius: 3px; }

  .attendees-section { margin-bottom: 1.5rem; }
  .attendees-section h3 { font-size: 14px; font-weight: 600; margin: 0 0 8px; color: var(--text-secondary); }
  .attendee-list { display: flex; flex-direction: column; gap: 4px; }
  .attendee { display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; border-radius: 4px; text-decoration: none; color: var(--text-primary); font-size: 13px; transition: background 0.1s; }
  .attendee:hover { background: var(--bg-hover); text-decoration: none; }
  .attendee-name { color: var(--accent); }
  .attendee-status { font-size: 11px; color: var(--text-hint); }

  .post-date { font-size: 12px; color: var(--text-hint); margin-bottom: 1rem; }

  .owner-actions { display: flex; gap: 8px; padding-top: 1rem; border-top: 1px solid var(--border); }
  .btn { padding: 6px 14px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; text-decoration: none; }
  .btn:hover { border-color: var(--accent); color: var(--accent); }
  .btn-primary { background: var(--accent); color: white; border-color: var(--accent); }
  .btn-primary:hover { opacity: 0.9; color: white; }
  .btn-danger { color: #dc2626; border-color: #dc2626; }
  .btn-danger:hover { background: #dc2626; color: white; }
</style>
