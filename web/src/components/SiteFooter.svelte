<script lang="ts">
  import type { LangTag } from "../lib/types";

  const GITHUB_REPO_URL = "https://github.com/sssxks/end-cli";

  function detectBrowserLang(): LangTag {
    if (typeof navigator === "undefined") {
      return "zh";
    }

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

  function t(zh: string, en: string): string {
    return lang === "zh" ? zh : en;
  }
</script>

<footer class="site-footer">
  <div class="inner">
    <div class="brand">
      <span class="title">{t("终末地产线规划", "Endfield Production Planner")}</span>
      <span class="dot" aria-hidden="true">·</span>
      <a class="link" href={GITHUB_REPO_URL} target="_blank" rel="noreferrer">
        {t("GitHub（反馈/问题）", "GitHub (feedback / issues)")}
      </a>
    </div>

    <nav class="nav" aria-label={t("页脚导航", "Footer navigation")}>
      <a class="link" href="#/">{t("工具", "App")}</a>
      <a class="link" href="#/about">{t("关于", "About")}</a>
      <a class="link" href="#/how">{t("它如何工作", "How it works")}</a>
    </nav>
  </div>
</footer>

<style>
  .site-footer {
    width: min(1800px, 100%);
    margin: 0 auto;
    padding: 0 var(--space-3) var(--space-3);
  }

  .inner {
    border-radius: var(--radius-md);
    background: var(--panel);
    box-shadow: var(--shadow-panel);
    border: 1px solid color-mix(in srgb, var(--line) 70%, var(--line-tint-1));
    padding: var(--space-3);

    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
    align-items: center;
  }

  .brand {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    flex-wrap: wrap;
  }

  .title {
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .dot {
    color: var(--muted-text);
  }

  .nav {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .link {
    color: var(--accent-ink);
    text-decoration: none;
    font-weight: 500;
  }

  .link:hover {
    text-decoration: underline;
  }
</style>
