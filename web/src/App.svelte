<script lang="ts">
  import { onMount } from "svelte";
  import SiteFooter from "./components/SiteFooter.svelte";
  import About from "./routes/About.svelte";
  import Home from "./routes/Home.svelte";
  import type { RouteKey } from "./lib/routes";
  import { parseHashRoute } from "./lib/routes";
  import "./styles/app-shell.css";

  let route = $state<RouteKey>(
    typeof window === "undefined"
      ? "home"
      : parseHashRoute(window.location.hash),
  );

  let howRouteModulePromise: Promise<typeof import("./routes/HowItWorks.svelte")> | null = null;

  function loadHowRoute() {
    howRouteModulePromise ??= import("./routes/HowItWorks.svelte");
    return howRouteModulePromise;
  }

  onMount(() => {
    const onHashChange = () => {
      route = parseHashRoute(window.location.hash);
    };

    onHashChange();
    window.addEventListener("hashchange", onHashChange);
    return () => {
      window.removeEventListener("hashchange", onHashChange);
    };
  });
</script>

<div class="shell">
  {#if route === "home"}
    <Home />
  {:else if route === "about"}
    <About />
  {:else}
    {#await loadHowRoute() then howModule}
      {@const HowItWorks = howModule.default}
      <HowItWorks />
    {/await}
  {/if}

  <SiteFooter />
</div>

<style>
  .shell {
    /* 1800px x 100dvh, padding var(--space-3) */
    width: min(1800px, 100%);
    margin: 0 auto;
    padding: var(--space-3);
    height: 100dvh;
    display: flex;
    flex-direction: column;

    /* for mobile tabs */
    gap: var(--space-3);
  }
</style>
