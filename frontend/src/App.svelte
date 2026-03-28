<script lang="ts">
  import './app.css';
  import Toast from './lib/components/Toast.svelte';
  import NavBar from './components/NavBar.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import RightSidebar from './components/RightSidebar.svelte';
  import KeyboardShortcuts from './components/KeyboardShortcuts.svelte';
  import Home from './routes/Home.svelte';
  import ArticlePage from './routes/Article.svelte';
  import NewArticle from './routes/NewArticle.svelte';
  import Skills from './routes/Skills.svelte';
  import About from './routes/About.svelte';
  import TagDetail from './routes/TagDetail.svelte';
  import Library from './routes/Library.svelte';
  import SeriesDetail from './routes/SeriesDetail.svelte';
  import NewSeries from './routes/NewSeries.svelte';
  import Profile from './routes/Profile.svelte';
  import SkillTreeView from './routes/SkillTreeView.svelte';
  import NewSkillTree from './routes/NewSkillTree.svelte';
  import Guide from './routes/Guide.svelte';
  import Roadmap from './routes/Roadmap.svelte';
  import Forks from './routes/Forks.svelte';
  import Drafts from './routes/Drafts.svelte';
  import Notifications from './routes/Notifications.svelte';
  import Questions from './routes/Questions.svelte';
  import QuestionDetail from './routes/QuestionDetail.svelte';
  import NewQuestion from './routes/NewQuestion.svelte';
  import Settings from './routes/Settings.svelte';
  import BookList from './routes/BookList.svelte';
  import BookDetailPage from './routes/BookDetail.svelte';

  let hash = $state(window.location.hash || '#/');

  function onHashChange() {
    hash = window.location.hash || '#/';
  }

  $effect(() => {
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  });

  function getRoute(h: string): { page: string; params: Record<string, string> } {
    const path = h.slice(1) || '/'; // remove '#'
    const [base, query] = path.split('?');
    const params: Record<string, string> = {};
    if (query) {
      for (const part of query.split('&')) {
        const [k, v] = part.split('=');
        params[decodeURIComponent(k)] = decodeURIComponent(v || '');
      }
    }
    if (base === '/' || base === '') return { page: 'home', params };
    if (base === '/tags') return { page: 'skills', params };
    if (base === '/article') return { page: 'article', params };
    if (base === '/new') return { page: 'new', params };
    if (base === '/skills') return { page: 'skills', params };
    if (base === '/graph') return { page: 'skills', params };
    if (base === '/tag') return { page: 'tag', params };
    if (base === '/about') return { page: 'about', params };
    if (base === '/roadmap') return { page: 'roadmap', params };
    if (base === '/guide') return { page: 'guide', params };
    if (base === '/library') return { page: 'library', params };
    if (base === '/series') return { page: 'series', params };
    if (base === '/new-series') return { page: 'new-series', params };
    if (base === '/profile') return { page: 'profile', params };
    if (base === '/skill-trees') return { page: 'skills', params };
    if (base === '/skill-tree/new') return { page: 'skill-tree-new', params };
    if (base === '/skill-tree') return { page: 'skill-tree', params };
    if (base === '/forks') return { page: 'forks', params };
    if (base === '/drafts') return { page: 'drafts', params };
    if (base === '/notifications') return { page: 'notifications', params };
    if (base === '/questions') return { page: 'questions', params };
    if (base === '/question') return { page: 'question', params };
    if (base === '/new-question') return { page: 'new-question', params };
    if (base === '/settings') return { page: 'settings', params };
    if (base === '/books') return { page: 'books', params };
    if (base === '/book') return { page: 'book', params };
    return { page: 'home', params };
  }

  let route = $derived(getRoute(hash));
  let kbShortcuts: KeyboardShortcuts;
</script>

<Toast />
<KeyboardShortcuts bind:this={kbShortcuts} />

{#if route.page === 'library'}
  <div class="fullwidth-nav">
    <NavBar />
  </div>
  <Library />
{:else if route.page === 'skills'}
  <div class="fullwidth-nav">
    <NavBar />
  </div>
  <Skills />
{:else if route.page === 'article'}
  <div class="top-nav">
    <NavBar />
  </div>
  <div class="container article-view">
    <ArticlePage uri={route.params.uri || ''} />
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
{:else if route.page !== 'book'}
  <div class="top-nav">
    <NavBar />
  </div>
  <div class="container">
    {#if route.page === 'tag'}
      <TagDetail id={route.params.id || ''} />
    {:else if route.page === 'new'}
      <NewArticle forkOf={route.params.fork_of || ''} editUri={route.params.edit || ''} draftId={route.params.draft || ''} initialCategory={route.params.category || ''} initialBookId={route.params.book_id || ''} />
    {:else if route.page === 'about'}
      <About />
    {:else if route.page === 'roadmap'}
      <Roadmap />
    {:else if route.page === 'guide'}
      <Guide />
    {:else if route.page === 'series'}
      <SeriesDetail id={route.params.id || ''} />
    {:else if route.page === 'new-series'}
      <NewSeries parentId={route.params.parent_id} />
    {:else if route.page === 'profile'}
      <Profile did={route.params.did || ''} />
    {:else if route.page === 'skill-tree'}
      <SkillTreeView uri={route.params.uri || ''} />
    {:else if route.page === 'skill-tree-new'}
      <NewSkillTree />
    {:else if route.page === 'forks'}
      <Forks uri={route.params.uri || ''} />
    {:else if route.page === 'drafts'}
      <Drafts />
    {:else if route.page === 'notifications'}
      <Notifications />
    {:else if route.page === 'questions'}
      <Questions />
    {:else if route.page === 'question'}
      <QuestionDetail uri={route.params.uri || ''} />
    {:else if route.page === 'new-question'}
      <NewQuestion />
    {:else if route.page === 'settings'}
      <Settings />
    {:else if route.page === 'books'}
      <BookList />
    {/if}
  </div>
{/if}

{#if route.page === 'book'}
  <div class="top-nav-wide">
    <NavBar />
  </div>
  <div class="container-wide">
    <BookDetailPage id={route.params.id || ''} />
  </div>
{/if}

<style>
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
