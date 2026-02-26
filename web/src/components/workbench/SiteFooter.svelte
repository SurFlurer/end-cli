<script lang="ts">
  import { onMount } from "svelte";
  import type { LangTag } from "../../lib/types";
  import type { RouteKey } from "../../lib/routes";
  import { parseHashRoute } from "../../lib/routes";
  import { tooltip } from "../../lib/tooltip";
  import FooterCollapseHandle from "./FooterCollapseHandle.svelte";
  import IconActionButton from "../button/IconActionButton.svelte";

  const GITHUB_REPO_URL = "https://github.com/sssxks/end-cli";
  const FOOTER_COLLAPSED_STORAGE_KEY = "end2.siteFooter.collapsed";

  function detectBrowserLang(): LangTag {
    const preferred = Array.isArray(navigator.languages)
      ? [...navigator.languages, navigator.language]
      : [navigator.language];

    for (const tag of preferred) {
      const normalized = tag.trim().toLowerCase();
      if (normalized.startsWith("zh")) {
        return "zh";
      }
      if (normalized.startsWith("en")) {
        return "en";
      }
    }

    return "zh";
  }

  let lang = $state<LangTag>(detectBrowserLang());

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

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }

  const tooltipGithub = () => t("反馈和问题", "Feedback and issues");
  const tooltipApp = () => t("返回应用首页", "Go to the app");
  const tooltipAbout = () => t("了解此项目", "About this project");
  const tooltipHow = () => t("了解它如何工作", "Learn how it works");

  let isCollapsed = $state(false);

  onMount(() => {
    try {
      const stored = window.localStorage.getItem(FOOTER_COLLAPSED_STORAGE_KEY);
      if (stored === "1" || stored === "true") {
        isCollapsed = true;
      }
      if (stored === "0" || stored === "false") {
        isCollapsed = false;
      }
    } catch {
      // ignore (private mode / storage disabled)
    }
  });

  $effect(() => {
    try {
      window.localStorage.setItem(FOOTER_COLLAPSED_STORAGE_KEY, isCollapsed ? "1" : "0");
    } catch {
      // ignore (private mode / storage disabled)
    }
  });

  const handleLabel = () =>
    isCollapsed ? t("展开页脚", "Expand footer") : t("折叠页脚", "Collapse footer");
</script>

<footer class:collapsed={isCollapsed}>
  <div class="brand">
    <FooterCollapseHandle
      isCollapsed={isCollapsed}
      label={handleLabel()}
      onToggle={() => {
        isCollapsed = !isCollapsed;
      }}
    />

    <span class="title">{t("源石计划", "end-cli")}</span>
    
    <span class="dot" aria-hidden="true">·</span>
    
    <span class="github-link">
      <IconActionButton
        ariaLabel={t("GitHub 仓库", "GitHub repository")}
        label={t("GitHub", "GitHub")}
        title={tooltipGithub()}
        href={GITHUB_REPO_URL}
        target="_blank"
        rel="noreferrer"
      >
        {#snippet iconSnippet()}
          <svg
            class="github-icon"
            viewBox="0 0 24 24"
            width="18"
            height="18"
            focusable="false"
            aria-hidden="true"
          >
            <path
              fill="currentColor"
              d="M12 2C6.477 2 2 6.586 2 12.253c0 4.532 2.865 8.376 6.839 9.733.5.096.682-.223.682-.495 0-.245-.009-.894-.014-1.754-2.782.62-3.369-1.377-3.369-1.377-.455-1.184-1.11-1.499-1.11-1.499-.908-.645.068-.632.068-.632 1.004.073 1.532 1.058 1.532 1.058.892 1.566 2.341 1.114 2.91.852.09-.666.349-1.114.635-1.37-2.222-.26-4.555-1.142-4.555-5.08 0-1.122.39-2.04 1.029-2.758-.103-.26-.446-1.307.098-2.723 0 0 .84-.276 2.75 1.053A9.324 9.324 0 0 1 12 6.997a9.29 9.29 0 0 1 2.504.348c1.909-1.329 2.748-1.053 2.748-1.053.546 1.416.203 2.463.1 2.723.64.718 1.028 1.636 1.028 2.758 0 3.948-2.337 4.817-4.566 5.072.359.317.679.944.679 1.903 0 1.374-.012 2.482-.012 2.819 0 .274.18.595.688.494C19.138 20.625 22 16.783 22 12.253 22 6.586 17.523 2 12 2Z"
            />
          </svg>
        {/snippet}
      </IconActionButton>
    </span>
  </div>

  <nav class="nav" aria-label={t("页脚导航", "Footer navigation")}>
    <IconActionButton
      ariaLabel={t("应用", "App")}
      icon="apps"
      label={t("应用", "App")}
      title={tooltipApp()}
      href="#/"
      active={route === "home"}
    />
    <IconActionButton
      ariaLabel={t("关于", "About")}
      icon="info"
      label={t("关于", "About")}
      title={tooltipAbout()}
      href="#/about"
      active={route === "about"}
    />
    <IconActionButton
      ariaLabel={t("它如何工作", "How it works")}
      icon="help"
      label={t("它如何工作", "How it works")}
      title={tooltipHow()}
      href="#/how"
      active={route === "how"}
    />
  </nav>
</footer>

<style>
  footer {
    border-radius: var(--radius-md);
    background: var(--panel);
    box-shadow: var(--shadow-panel);
    padding: var(--space-1) var(--space-3);

    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
    align-items: center;
  }

  footer.collapsed {
    position: fixed;
    left: var(--space-3);
    bottom: var(--space-3);
    z-index: 20;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    flex-wrap: wrap;
  }

  footer.collapsed .title {
    display: none;
  }
  footer.collapsed .dot,
  footer.collapsed .github-link,
  footer.collapsed .nav {
    display: none;
  }

  .title {
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .dot {
    color: var(--muted-text);
  }

  .github-link {
    color: var(--accent-ink);
  }

  .nav {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
    color: var(--accent-ink);
  }

  .github-icon {
    display: block;
  }
</style>
