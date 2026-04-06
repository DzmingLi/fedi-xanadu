<script lang="ts">
  import './app.css';
  import Toast from './lib/components/Toast.svelte';
  import NavBar from './components/NavBar.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import RightSidebar from './components/RightSidebar.svelte';
  import KeyboardShortcuts from './components/KeyboardShortcuts.svelte';
  import Home from './routes/Home.svelte';
  import { matchRoute } from './lib/router';

  let hash = $state(window.location.hash || '#/');

  function onHashChange() {
    hash = window.location.hash || '#/';
  }

  $effect(() => {
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  });

  let route = $derived(matchRoute(hash));
  let kbShortcuts: KeyboardShortcuts;

  // Lazy-load route components (not on the home page)
  const lazyArticle = () => import('./routes/Article.svelte');
  const lazyNewArticle = () => import('./routes/NewArticle.svelte');
  const lazySkills = () => import('./routes/Skills.svelte');
  const lazyAbout = () => import('./routes/About.svelte');
  const lazyTagDetail = () => import('./routes/TagDetail.svelte');
  const lazyLibrary = () => import('./routes/Library.svelte');
  const lazySeriesDetail = () => import('./routes/SeriesDetail.svelte');
  const lazyNewSeries = () => import('./routes/NewSeries.svelte');
  const lazyProfile = () => import('./routes/Profile.svelte');
  const lazySkillTreeView = () => import('./routes/SkillTreeView.svelte');
  const lazyNewSkillTree = () => import('./routes/NewSkillTree.svelte');
  const lazyGuide = () => import('./routes/Guide.svelte');
  const lazyRoadmap = () => import('./routes/Roadmap.svelte');
  const lazyForks = () => import('./routes/Forks.svelte');
  const lazyDrafts = () => import('./routes/Drafts.svelte');
  const lazyNotifications = () => import('./routes/Notifications.svelte');
  const lazyQuestions = () => import('./routes/Questions.svelte');
  const lazyQuestionDetail = () => import('./routes/QuestionDetail.svelte');
  const lazyNewQuestion = () => import('./routes/NewQuestion.svelte');
  const lazySettings = () => import('./routes/Settings.svelte');
  const lazyBookList = () => import('./routes/BookList.svelte');
  const lazyBookDetail = () => import('./routes/BookDetail.svelte');
  const lazyBookEdition = () => import('./routes/BookEdition.svelte');
</script>

<Toast />
<KeyboardShortcuts bind:this={kbShortcuts} />

{#if route.page === 'library'}
  <div class="fullwidth-nav">
    <NavBar />
  </div>
  {#await lazyLibrary() then mod}
    <mod.default />
  {/await}
{:else if route.page === 'skills'}
  <div class="fullwidth-nav">
    <NavBar />
  </div>
  {#await lazySkills() then mod}
    <mod.default />
  {/await}
{:else if route.page === 'article'}
  <div class={route.params.series_id ? 'top-nav-series' : 'top-nav'}>
    <NavBar />
  </div>
  <div class={route.params.series_id ? 'container-series' : 'container article-view'}>
    {#await lazyArticle() then mod}
      <mod.default uri={route.params.uri || ''} seriesId={route.params.series_id || ''} />
    {/await}
  </div>
{:else if route.page === 'home'}
  <div class="layout-wide">
    <NavBar />
    <div class="layout-body">
      <Sidebar />
      <main class="layout-main">
        <Home />
      </main>
      <RightSidebar />
    </div>
  </div>
{:else if route.page === 'new'}
  <div class="editor-nav">
    <NavBar />
  </div>
  <div class="editor-container">
    {#await lazyNewArticle() then mod}
      <mod.default forkOf={route.params.fork_of || ''} editUri={route.params.edit || ''} draftId={route.params.draft || ''} initialCategory={route.params.category || ''} initialBookId={route.params.book_id || ''} />
    {/await}
  </div>
{:else if route.page !== 'book'}
  <div class="top-nav">
    <NavBar />
  </div>
  <div class="container">
    {#if route.page === 'tag'}
      {#await lazyTagDetail() then mod}
        <mod.default id={route.params.id || ''} />
      {/await}
    {:else if route.page === 'about'}
      {#await lazyAbout() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'roadmap'}
      {#await lazyRoadmap() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'guide'}
      {#await lazyGuide() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'series'}
      {#await lazySeriesDetail() then mod}
        <mod.default id={route.params.id || ''} />
      {/await}
    {:else if route.page === 'new-series'}
      {#await lazyNewSeries() then mod}
        <mod.default parentId={route.params.parent_id} />
      {/await}
    {:else if route.page === 'profile'}
      {#await lazyProfile() then mod}
        <mod.default did={route.params.did || ''} />
      {/await}
    {:else if route.page === 'skill-tree'}
      {#await lazySkillTreeView() then mod}
        <mod.default uri={route.params.uri || ''} />
      {/await}
    {:else if route.page === 'skill-tree-new'}
      {#await lazyNewSkillTree() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'forks'}
      {#await lazyForks() then mod}
        <mod.default uri={route.params.uri || ''} />
      {/await}
    {:else if route.page === 'drafts'}
      {#await lazyDrafts() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'notifications'}
      {#await lazyNotifications() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'questions'}
      {#await lazyQuestions() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'question'}
      {#await lazyQuestionDetail() then mod}
        <mod.default uri={route.params.uri || ''} />
      {/await}
    {:else if route.page === 'new-question'}
      {#await lazyNewQuestion() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'settings'}
      {#await lazySettings() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'books'}
      {#await lazyBookList() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'book-edition'}
      {#await lazyBookEdition() then mod}
        <mod.default bookId={route.params.book_id || ''} />
      {/await}
    {/if}
  </div>
{/if}

{#if route.page === 'book'}
  <div class="top-nav-wide">
    <NavBar />
  </div>
  <div class="container-wide">
    {#await lazyBookDetail() then mod}
      <mod.default id={route.params.id || ''} />
    {/await}
  </div>
{/if}

<style>
  .editor-nav {
    padding: 0 1rem;
    max-width: 100%;
  }
  .editor-container {
    max-width: 100%;
    height: calc(100vh - 3.5rem);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .top-nav-series {
    max-width: 1100px;
    margin: 0 auto;
    padding: 0 1rem;
  }
  .container-series {
    max-width: 1100px;
    margin: 0 auto;
    padding: 0 1rem;
    display: flex;
    gap: 0;
  }
  .top-nav {
    max-width: 760px;
    margin: 0 auto;
    padding: 0 1rem;
  }
  .fullwidth-nav {
    padding: 0 1rem;
  }
  .layout-wide {
    max-width: 1280px;
    margin: 0 auto;
    padding: 0 1rem;
  }
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
