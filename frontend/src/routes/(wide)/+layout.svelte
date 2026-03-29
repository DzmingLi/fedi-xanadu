<script lang="ts">
  import NavBar from '$lib/components/NavBar.svelte';
  let { children } = $props();
</script>

<div class="top-nav-wide">
  <NavBar />
</div>
<div class="container-wide">
  {@render children()}
</div>

<style>
  .top-nav-wide {
    max-width: 1080px;
    margin: 0 auto;
    padding: 0 1rem;
  }
  .container-wide {
    max-width: 1080px;
    margin: 0 auto;
    padding: 0 1rem;
  }
</style>
