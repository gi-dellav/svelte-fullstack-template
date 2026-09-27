<script lang="ts">
  import { onMount } from "svelte";
  import PwaUpdate from "./PwaUpdate.svelte";
  import Post from "./routes/Post.svelte";
  import Seo from "./Seo.svelte";
  import { getPost, posts } from "./lib/posts";
  import { apiUrl } from "./lib/api";
  import {
    errorState,
    fetchJson,
    idleState,
    loadingState,
    okState,
    toErrorMessage,
    type AsyncState,
  } from "./lib/async";
  import {
    currentRoute,
    handleLinkClick,
    withBase,
    type Route,
  } from "./lib/router";

  let route = $state<Route>({ name: "home" });
  let activePost = $derived(route.name === "post" ? getPost(route.slug) : undefined);

  onMount(() => {
    route = currentRoute();
    const onPopState = () => {
      route = currentRoute();
      window.scrollTo({ top: 0 });
    };
    const onClick = (event: MouseEvent) => handleLinkClick(event);
    window.addEventListener("popstate", onPopState);
    document.addEventListener("click", onClick);
    return () => {
      window.removeEventListener("popstate", onPopState);
      document.removeEventListener("click", onClick);
    };
  });

  let count = $state(0);

  interface Health {
    status: string;
  }

  let health = $state<AsyncState<Health>>(idleState());

  function loadHealth() {
    let cancelled = false;
    health = loadingState();
    fetchJson<Health>(apiUrl("/api/health"))
      .then((data) => {
        if (!cancelled) health = okState(data);
      })
      .catch((error: unknown) => {
        if (!cancelled) health = errorState(toErrorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }

  const stack = ["Svelte 5", "Vite", "Tailwind CSS 4", "PWA", "Bun", "Axum", "Vercel"];
  const commands = `cd frontend && bun install  # install frontend deps
bun run dev                # start the dev server (proxies /api → :3000)
bun run check              # type-check with svelte-check
bun run build              # static production build into dist/
cd ../backend && cargo run # start the Axum API on :3000`;
</script>

{#if route.name === "post"}
  {#if activePost}
    <Seo
      title={activePost.metadata.title}
      description={activePost.metadata.description}
      path={`/post/${activePost.slug}/`}
    />
    <Post post={activePost} />
  {:else}
    <Seo title="Post not found" path="/" noindex={true} />
    <main class="page">
      <p class="eyebrow">404</p>
      <h1 class="title">Post not found.</h1>
      <div class="actions">
        <a href={withBase("/")} class="btn-primary">Back home</a>
      </div>
    </main>
  {/if}
{:else if route.name === "not-found"}
  {@const missingPath = route.path}
  <Seo title="Page not found" path={missingPath} noindex={true} />
  <main class="page">
    <p class="eyebrow">404</p>
    <h1 class="title">Page not found.</h1>
    <p class="lede">No page at <code class="code">{missingPath}</code>.</p>
    <div class="actions">
      <a href={withBase("/")} class="btn-primary">Back home</a>
    </div>
  </main>
{:else}
  <Seo path="/" />
  <main class="page">
  <p class="eyebrow">svelte-fullstack-template</p>

  <h1 class="title">Clean frontend, Axum backend.</h1>
  <p class="lede">
    A minimalist fullstack starter: Svelte + Tailwind frontend, Axum backend.
    Push to <span class="font-medium text-neutral-900">main</span> and Vercel
    ships both Docker services.
  </p>

  <div class="meta">
    {#each stack as name, i (name)}
      <span>{name}</span>
      {#if i < stack.length - 1}
        <span aria-hidden="true" class="dot"></span>
      {/if}
    {/each}
  </div>

  <div class="actions">
    <button type="button" class="btn-primary" onclick={() => (count += 1)}>
      Clicked {count} {count === 1 ? "time" : "times"}
    </button>
    <a href={withBase("/post/hello")} class="link-quiet">Read the sample post</a>
  </div>

  <section aria-labelledby="start-heading" class="section">
    <h2 id="start-heading" class="h2">Get started</h2>
    <p class="body">
      Frontend runs on <a
        href="https://bun.sh"
        target="_blank"
        rel="noreferrer"
        class="link-inline">Bun</a
      >, backend on <a
        href="https://www.rust-lang.org"
        target="_blank"
        rel="noreferrer"
        class="link-inline">Rust</a
      > — run both:
    </p>
    <pre class="pre"><code>{commands}</code></pre>
    <p class="body">
      Edit <code class="code">frontend/src/App.svelte</code> to make it yours. The
      <code class="code">frontend/dist/</code> folder is the static frontend bundle —
      preview it with <code class="code">bun run preview</code>.
    </p>
  </section>

  <section aria-labelledby="writing-heading" class="section">
    <h2 id="writing-heading" class="h2">Writing</h2>
    <p class="body">
      Posts live in <code class="code">frontend/src/content/*.md</code> — compiled to
      HTML at build time, with frontmatter and GFM (tables, task lists).
    </p>
    <ol class="steps">
      {#each posts as post (post.slug)}
        <li>
          <a href={withBase(`/post/${post.slug}`)} class="link-inline">
            {post.metadata.title}
          </a>
          <span class="step-body">
            {post.metadata.description ?? post.slug}
            {#if post.metadata.date} · {post.metadata.date}{/if}
          </span>
        </li>
      {/each}
    </ol>
  </section>

  <section aria-labelledby="deploy-heading" class="section">
    <h2 id="deploy-heading" class="h2">Deploy to Vercel</h2>
    <p class="body">
      One project, two Docker services: <code class="code">frontend</code> (static build)
      and <code class="code">backend</code> (Axum container). Push to
      <code class="code">main</code> and Vercel builds both services.
    </p>
    <ol class="steps">
      <li>
        <span class="step-title">Import the repo.</span>
        <span class="step-body"
          >In Vercel, <strong>Add New → Project → Import</strong> this repo. No root
          directory override needed — <code class="code">vercel.json</code> declares
          both services. Details: <code class="code">knowledge/vercel-deploy.md</code>.</span
        >
      </li>
      <li>
        <span class="step-title">Set SITE_URL.</span>
        <span class="step-body"
          >Add <code class="code">SITE_URL=https://&lt;project&gt;.vercel.app/</code>
          so <code class="code">sitemap.xml</code> carries absolute URLs.</span
        >
      </li>
      <li>
        <span class="step-title">Push to main.</span>
        <span class="step-body"
          >Every push gets Preview URLs per service; production deploys from
          <code class="code">main</code>. Health check lives at
          <code class="code">/api/health</code>.</span
        >
      </li>
    </ol>
  </section>

  <section aria-labelledby="api-heading" class="section">
    <h2 id="api-heading" class="h2">Backend</h2>
    <p class="body">
      The Axum API runs alongside this page (SQLite by default — see
      <code class="code">knowledge/database.md</code>). Start it with
      <code class="code">cargo run</code> in <code class="code">backend/</code>,
      then check its health:
    </p>
    <div class="actions">
      <button type="button" class="btn-primary" onclick={loadHealth}>Check API health</button>
      {#if health.status === "ok"}
        <span class="note" role="status">API says: {health.data.status}</span>
      {:else if health.status === "loading"}
        <span class="hint">Checking…</span>
      {:else if health.status === "error"}
        <span class="form-error" role="alert">{health.error}</span>
      {:else}
        <span class="hint">GET /api/health</span>
      {/if}
    </div>
  </section>

  <p class="footnote">Edit <span class="text-neutral-500">frontend/src/App.svelte</span> to get started</p>

  <footer class="footer">
    <span class="hint">Svelte 5 + Axum fullstack template — deploy with Vercel (`vercel.json`).</span>
  </footer>
  </main>
{/if}

<PwaUpdate />
