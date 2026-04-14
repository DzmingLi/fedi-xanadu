<script lang="ts">
  import { getListing, closeListing, reopenListing, deleteListing } from '../lib/api';
  import { getAuth } from '../lib/auth.svelte';
  import { fmtRep } from '../lib/display';
  import type { Listing, ListingKind } from '../lib/types';

  const KIND_LABELS: Record<ListingKind, string> = {
    phd: 'PhD', masters: 'Masters', ra: 'Research Assistant',
    postdoc: 'Postdoc', intern: 'Internship', faculty: 'Faculty', other: 'Other',
  };

  let { id }: { id: string } = $props();
  let listing = $state<Listing | null>(null);
  let error = $state('');
  let isOwner = $derived(!!getAuth() && listing?.did === getAuth()?.did);

  $effect(() => {
    if (!id) return;
    getListing(id).then(l => {
      listing = l;
      document.title = `${l.title} — NightBoat`;
    }).catch(e => { error = e.message; });
  });

  async function doClose() {
    if (!listing || !confirm('Close this listing?')) return;
    await closeListing(listing.id);
    listing = { ...listing, is_open: false };
  }
  async function doReopen() {
    if (!listing) return;
    await reopenListing(listing.id);
    listing = { ...listing, is_open: true };
  }
  async function doDelete() {
    if (!listing || !confirm('Permanently delete this listing?')) return;
    await deleteListing(listing.id);
    window.location.href = '/listings';
  }
</script>

{#if error}
  <div class="error">{error}</div>
{:else if !listing}
  <p>Loading...</p>
{:else}
  <article class="listing-detail">
    <div class="header">
      <span class="kind-badge">{KIND_LABELS[listing.kind] || listing.kind}</span>
      {#if !listing.is_open}<span class="closed-badge">Closed</span>{/if}
    </div>

    <h1>{listing.title}</h1>

    <div class="meta-row">
      <a href="/profile?did={encodeURIComponent(listing.did)}" class="author">
        @{listing.author_handle || listing.did.slice(0, 16)}
      </a>
      {#if listing.author_reputation > 0}
        <span class="rep">{fmtRep(listing.author_reputation)}</span>
      {/if}
      <span class="dot">·</span>
      <span>{listing.institution}</span>
      {#if listing.department}
        <span class="dot">·</span>
        <span>{listing.department}</span>
      {/if}
    </div>

    <div class="info-grid">
      {#if listing.location}
        <div class="info-item"><strong>Location</strong><span>{listing.location}</span></div>
      {/if}
      <div class="info-item">
        <strong>Deadline</strong>
        <span>{listing.deadline ? new Date(listing.deadline).toLocaleDateString() : 'Rolling'}</span>
      </div>
      {#if listing.compensation}
        <div class="info-item"><strong>Compensation</strong><span>{listing.compensation}</span></div>
      {/if}
      {#if listing.contact_email}
        <div class="info-item"><strong>Contact</strong><a href="mailto:{listing.contact_email}">{listing.contact_email}</a></div>
      {/if}
      {#if listing.contact_url}
        <div class="info-item"><strong>Link</strong><a href={listing.contact_url} target="_blank" rel="noopener">{listing.contact_url}</a></div>
      {/if}
    </div>

    {#if listing.required_tags.length > 0 || listing.preferred_tags.length > 0}
      <div class="skills-section">
        {#if listing.required_tags.length > 0}
          <div class="skill-group">
            <h3>Required Skills</h3>
            <div class="tags">
              {#each listing.required_tags as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag required">{tag}</a>
              {/each}
            </div>
          </div>
        {/if}
        {#if listing.preferred_tags.length > 0}
          <div class="skill-group">
            <h3>Preferred Skills</h3>
            <div class="tags">
              {#each listing.preferred_tags as tag}
                <a href="/tag?id={encodeURIComponent(tag)}" class="tag preferred">{tag}</a>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if listing.description}
      <div class="description">{listing.description}</div>
    {/if}

    <div class="post-date">Posted {new Date(listing.created_at).toLocaleDateString()}</div>

    {#if isOwner}
      <div class="owner-actions">
        <a href="/new-listing?edit={encodeURIComponent(listing.id)}" class="btn">Edit</a>
        {#if listing.is_open}
          <button class="btn" onclick={doClose}>Close</button>
        {:else}
          <button class="btn" onclick={doReopen}>Reopen</button>
        {/if}
        <button class="btn btn-danger" onclick={doDelete}>Delete</button>
      </div>
    {/if}
  </article>
{/if}

<style>
  .listing-detail { max-width: 700px; }
  .error { background: #fef2f2; color: #dc2626; padding: 12px; border-radius: 4px; }

  .header { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; }
  .kind-badge { font-size: 12px; font-weight: 600; text-transform: uppercase; padding: 3px 8px; border-radius: 3px; background: var(--bg-page); border: 1px solid var(--border); color: var(--text-secondary); }
  .closed-badge { font-size: 12px; font-weight: 600; padding: 3px 8px; border-radius: 3px; background: #fef2f2; color: #dc2626; }

  h1 { font-family: var(--font-serif); font-weight: 400; font-size: 1.5rem; margin: 0 0 8px; }

  .meta-row { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: 14px; color: var(--text-secondary); margin-bottom: 1.5rem; }
  .author { color: var(--accent); text-decoration: none; }
  .author:hover { text-decoration: underline; }
  .rep { font-size: 11px; font-weight: 600; color: var(--text-secondary); background: var(--bg-page); border: 1px solid var(--border); border-radius: 3px; padding: 0 4px; }
  .dot { color: var(--text-hint); }

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
  .tag.required { background: #fef3c7; color: #92400e; }
  .tag.preferred { background: #e0e7ff; color: #3730a3; }

  .description { font-size: 15px; line-height: 1.7; white-space: pre-wrap; margin-bottom: 1.5rem; }

  .post-date { font-size: 12px; color: var(--text-hint); margin-bottom: 1rem; }

  .owner-actions { display: flex; gap: 8px; padding-top: 1rem; border-top: 1px solid var(--border); }
  .btn { padding: 6px 14px; font-size: 13px; border: 1px solid var(--border); border-radius: 3px; background: none; color: var(--text-secondary); cursor: pointer; text-decoration: none; }
  .btn:hover { border-color: var(--accent); color: var(--accent); }
  .btn-danger { color: #dc2626; border-color: #dc2626; }
  .btn-danger:hover { background: #dc2626; color: white; }
</style>
