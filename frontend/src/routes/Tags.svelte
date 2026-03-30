<script lang="ts">
  import { listTags } from '../lib/api';
  import type { Tag } from '../lib/types';

  let tags = $state<Tag[]>([]);
  let loading = $state(true);

  $effect(() => {
    listTags().then(data => {
      tags = data;
      loading = false;
    });
  });
</script>

<h1>Tags</h1>

{#if loading}
  <p class="meta">Loading...</p>
{:else if tags.length === 0}
  <div class="empty"><p>No tags yet.</p></div>
{:else}
  {#each tags as t}
    <div class="card">
      <h2><span class="tag">{t.id}</span> {t.name}</h2>
      <p class="meta">{t.description || ''}</p>
    </div>
  {/each}
{/if}
