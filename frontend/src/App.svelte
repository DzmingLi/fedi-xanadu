<script lang="ts">
  import './app.css';
  import Toast from './lib/components/Toast.svelte';
  import NavBar from './components/NavBar.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import RightSidebar from './components/RightSidebar.svelte';
  import KeyboardShortcuts from './components/KeyboardShortcuts.svelte';
  import Footer from './components/Footer.svelte';
  import Home from './routes/Home.svelte';
  import { matchRoute } from './lib/router';

  let currentPath = $state(window.location.pathname + window.location.search);

  function onNavigate() {
    currentPath = window.location.pathname + window.location.search;
  }

  $effect(() => {
    window.addEventListener('popstate', onNavigate);
    // Intercept all link clicks to use history API instead of full reload
    function onClick(e: MouseEvent) {
      const a = (e.target as HTMLElement).closest('a[href]') as HTMLAnchorElement | null;
      if (!a || a.target || a.hasAttribute('download') || e.ctrlKey || e.metaKey || e.shiftKey) return;
      const href = a.getAttribute('href');
      if (!href || href.startsWith('http') || href.startsWith('mailto:') || href.startsWith('javascript:')) return;
      if (href.startsWith('/') && !href.startsWith('/api/') && !href.startsWith('/oauth/')) {
        e.preventDefault();
        history.pushState(null, '', href);
        onNavigate();
      }
    }
    document.addEventListener('click', onClick);
    return () => {
      window.removeEventListener('popstate', onNavigate);
      document.removeEventListener('click', onClick);
    };
  });

  let route = $derived(matchRoute(currentPath));
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
  const lazySeriesEditor = () => import('./routes/SeriesEditor.svelte');
  const lazyDiscussion = () => import('./routes/Discussion.svelte');
  const lazyCreator = () => import('./routes/CreatorDashboard.svelte');
  const lazyThoughts = () => import('./routes/Thoughts.svelte');
  const lazyGuidelines = () => import('./routes/Guidelines.svelte');
  const lazyFeedback = () => import('./routes/Feedback.svelte');
  const lazyAdmin = () => import('./routes/Admin.svelte');
  const lazyListings = () => import('./routes/Listings.svelte');
  const lazyListingDetail = () => import('./routes/ListingDetail.svelte');
  const lazyNewListing = () => import('./routes/NewListing.svelte');
  const lazyEvents = () => import('./routes/Events.svelte');
  const lazyEventDetail = () => import('./routes/EventDetail.svelte');
  const lazyNewEvent = () => import('./routes/NewEvent.svelte');
  const lazyCourses = () => import('./routes/Courses.svelte');
  const lazyCourseDetail = () => import('./routes/CourseDetail.svelte');
  const lazyNewCourse = () => import('./routes/NewCourse.svelte');
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
  <div class="top-nav">
    <NavBar />
  </div>
  <div class="container article-view">
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
{:else if route.page === 'series-editor'}
  <div class="editor-nav">
    <NavBar />
  </div>
  <div class="editor-container">
    {#await lazySeriesEditor() then mod}
      <mod.default id={route.params.id || ''} />
    {/await}
  </div>
{:else if route.page === 'profile'}
  <div class="profile-nav">
    <NavBar />
  </div>
  <div class="profile-container">
    {#await lazyProfile() then mod}
      <mod.default did={route.params.did || ''} />
    {/await}
  </div>
{:else if route.page === 'course-detail'}
  <div class="profile-nav">
    <NavBar />
  </div>
  <div class="profile-container">
    {#await lazyCourseDetail() then mod}
      <mod.default id={route.params.id || ''} />
    {/await}
  </div>
{:else if route.page === 'courses' || route.page === 'new-course'}
  <div class="profile-nav">
    <NavBar />
  </div>
  <div class="profile-container">
    {#if route.page === 'courses'}
      {#await lazyCourses() then mod}
        <mod.default />
      {/await}
    {:else}
      {#await lazyNewCourse() then mod}
        <mod.default />
      {/await}
    {/if}
  </div>
{:else if route.page !== 'book' && route.page !== 'books'}
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
        <mod.default />
      {/await}
    {:else if route.page === 'skill-tree'}
      {#await lazySkillTreeView() then mod}
        <mod.default uri={route.params.uri || ''} />
      {/await}
    {:else if route.page === 'skill-tree-new'}
      {#await lazyNewSkillTree() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'discussion'}
      {#await lazyDiscussion() then mod}
        <mod.default id={route.params.id || ''} />
      {/await}
    {:else if route.page === 'forks'}
      {#await lazyForks() then mod}
        <mod.default uri={route.params.uri || ''} />
      {/await}
    {:else if route.page === 'drafts'}
      {#await lazyDrafts() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'creator'}
      {#await lazyCreator() then mod}
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
    {:else if route.page === 'book-edition'}
      {#await lazyBookEdition() then mod}
        <mod.default bookId={route.params.book_id || ''} />
      {/await}
    {:else if route.page === 'thoughts'}
      {#await lazyThoughts() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'listings'}
      {#await lazyListings() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'listing-detail'}
      {#await lazyListingDetail() then mod}
        <mod.default id={route.params.id || ''} />
      {/await}
    {:else if route.page === 'new-listing'}
      {#await lazyNewListing() then mod}
        <mod.default editId={route.params.edit || ''} />
      {/await}
    {:else if route.page === 'events'}
      {#await lazyEvents() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'event-detail'}
      {#await lazyEventDetail() then mod}
        <mod.default id={route.params.id || ''} />
      {/await}
    {:else if route.page === 'new-event'}
      {#await lazyNewEvent() then mod}
        <mod.default editId={route.params.edit || ''} />
      {/await}
    {:else if route.page === 'feedback'}
      {#await lazyFeedback() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'guidelines'}
      {#await lazyGuidelines() then mod}
        <mod.default />
      {/await}
    {:else if route.page === 'admin'}
      {#await lazyAdmin() then mod}
        <mod.default />
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

{#if route.page === 'books'}
  <div class="top-nav-wide">
    <NavBar />
  </div>
  <div class="container-wide">
    {#await lazyBookList() then mod}
      <mod.default />
    {/await}
  </div>
{/if}

{#if route.page !== 'new' && route.page !== 'series-editor'}
  <Footer />
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
  .profile-nav {
    max-width: 1080px;
    margin: 0 auto;
    padding: 0 1rem;
  }
  .profile-container {
    max-width: 1080px;
    margin: 0 auto;
    padding: 0 1rem;
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
