<script lang="ts">
  import NavBar from '$lib/components/NavBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import RightSidebar from '$lib/components/RightSidebar.svelte';
  let { children } = $props();
</script>

<div class="layout-wide">
  <NavBar />
  <div class="layout-body">
    <Sidebar />
    <main class="layout-main">
      {@render children()}
    </main>
    <RightSidebar />
  </div>
</div>

<style>
  .layout-wide {
    max-width: 1280px;
    margin: 0 auto;
    padding: 0 1rem;
  }
  .layout-body {
    display: flex;
    gap: 2rem;
    padding-top: 0.5rem;
  }
  .layout-main {
    flex: 1;
    max-width: 760px;
    min-width: 0;
  }
  @media (max-width: 960px) {
    .layout-wide {
      max-width: 760px;
    }
  }
</style>
