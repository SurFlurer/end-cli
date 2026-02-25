<script lang="ts">
  import { onMount } from "svelte";
  import SiteFooter from "./components/SiteFooter.svelte";
  import About from "./routes/About.svelte";
  import HowItWorks from "./routes/HowItWorks.svelte";
  import Home from "./routes/Home.svelte";

  type RouteKey = "home" | "about" | "how";

  function parseHashRoute(hash: string): RouteKey {
    const raw = hash.trim();
    if (raw.length === 0) {
      return "home";
    }

    const cleaned = raw.startsWith("#") ? raw.slice(1) : raw;
    const pathname = cleaned.startsWith("/") ? cleaned.slice(1) : cleaned;

    if (pathname === "" || pathname === "/") {
      return "home";
    }

    const firstSegment = pathname.split("/")[0]?.toLowerCase() ?? "";
    if (firstSegment === "about") {
      return "about";
    }
    if (firstSegment === "how" || firstSegment === "how-it-works") {
      return "how";
    }

    return "home";
  }

  let route = $state<RouteKey>(
    typeof window === "undefined" ? "home" : parseHashRoute(window.location.hash),
  );

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

{#if route === "home"}
  <Home />
{:else if route === "about"}
  <About />
{:else}
  <HowItWorks />
{/if}

<SiteFooter />
