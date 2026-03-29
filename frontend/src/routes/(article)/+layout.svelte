<script lang="ts">
  import NavBar from '$lib/components/NavBar.svelte';
  let { children } = $props();
</script>

<div class="top-nav">
  <NavBar />
</div>
<div class="container article-view">
  {@render children()}
</div>

<style>
  .top-nav {
    max-width: 760px;
    margin: 0 auto;
    padding: 0 1rem;
  }
</style>
