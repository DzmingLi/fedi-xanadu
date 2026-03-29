<script lang="ts">
  import NavBar from '$lib/components/NavBar.svelte';
  let { children } = $props();
</script>

<div class="fullwidth-nav">
  <NavBar />
</div>
{@render children()}

<style>
  .fullwidth-nav {
    padding: 0 1rem;
  }
</style>
