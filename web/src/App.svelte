<script lang="ts">
  import { onMount } from "svelte";
  import SiteFooter from "./components/workbench/SiteFooter.svelte";
  import About from "./routes/About.svelte";
  import Home from "./routes/Home.svelte";
  import { detectBrowserLang } from "./lib/lang";
  import type { RouteKey } from "./lib/routes";
  import { getCurrentHashRoute, observeHashRoute } from "./lib/route-state";
  import type { LangTag } from "./lib/types";
  import "./styles/app-shell.css";

  let route = $state<RouteKey>(getCurrentHashRoute());
  let lang = $state<LangTag>(detectBrowserLang());

  let howRouteModulePromise: Promise<typeof import("./routes/HowItWorks.svelte")> | null = null;

  function loadHowRoute() {
    howRouteModulePromise ??= import("./routes/HowItWorks.svelte");
    return howRouteModulePromise;
  }

  onMount(() => {
    return observeHashRoute((nextRoute) => {
      route = nextRoute;
    });
  });
</script>

<div class="shell">
  {#if route === "home"}
    <Home {lang} />
  {:else if route === "about"}
    <About {lang} />
  {:else}
    {#await loadHowRoute() then howModule}
      {@const HowItWorks = howModule.default}
      <HowItWorks {lang} />
    {/await}
  {/if}

  <SiteFooter {lang} />
</div>

<style>
  .shell {
    margin: 0 auto;
    padding: var(--space-3);
    height: 100dvh;
    display: flex;
    flex-direction: column;

    /* for mobile tabs */
    gap: var(--space-3);
  }
</style>
