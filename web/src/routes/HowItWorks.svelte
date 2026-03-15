<script lang="ts">
  import "katex/dist/katex.min.css";
  import CopyButton from "../components/button/CopyButton.svelte";
  import Panel from "../components/pane/Panel.svelte";
  import PanelHeader from "../components/pane/PanelHeader.svelte";
  import {
    MODEL_V1_RENDERED_HTML,
    MODEL_V1_SOURCE_MARKDOWN,
  } from "../lib/generated/modelV1";
  import { translateByLang } from "../lib/lang";
  import type { LangTag } from "../lib/types";

  interface Props {
    lang: LangTag;
  }

  let { lang }: Props = $props();

  function t(zh: string, en: string): string {
    return translateByLang(lang, zh, en);
  }
</script>

<Panel>
  {#snippet header()}
    <PanelHeader
      titleText={t("这个 App 如何工作", "How this app works")}
      icon="psychology"
    >
      {#snippet controls()}
        <CopyButton {lang} text={MODEL_V1_SOURCE_MARKDOWN} />
      {/snippet}
    </PanelHeader>
  {/snippet}

  <article class="content">
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted static HTML generated from in-repo markdown -->
    {@html MODEL_V1_RENDERED_HTML}
  </article>
</Panel>

<style>
  .content {
    width: 100%;
    max-width: 960px;
    margin: 0 auto;

    color: var(--ink);
    line-height: 1.68;
    word-break: break-word;
  }

  @media (min-width: 760px) {
    .content {
      font-size: 16px;
    }
  }

  @media (max-width: 760px) {
    .content {
      font-size: 14px;
    }
  }

  /* The Markdown HTML is injected via {@html ...}, so styles must be global. */
  .content :global(.katex) {
    /* open PR as 2026/02/26: https://github.com/KaTeX/KaTeX/pull/3859 */
    /* katex hidden mathml uses absolute, cause issue when parent scrolling element is non-static */
    /* here we do a minimal fix by setting .katex position to relative */
    /* 2 hr and lots of tokens spent debugging this --xks */
    position: relative;
  }

  .content :global(:first-child) {
    margin-top: 0;
  }

  .content :global(:last-child) {
    margin-bottom: 0;
  }

  .content :global(p) {
    margin: var(--space-3) 0;
  }

  .content :global(h1),
  .content :global(h2),
  .content :global(h3),
  .content :global(h4) {
    margin: var(--space-5) 0 var(--space-3);
    line-height: 1.25;
    letter-spacing: -0.01em;
  }

  .content :global(h2) {
    padding-bottom: var(--space-2);
    border-bottom: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
  }

  .content :global(h3) {
    margin-top: var(--space-4);
  }

  .content :global(ul),
  .content :global(ol) {
    margin: var(--space-3) 0;
    padding-left: 1.25em;
  }

  .content :global(li) {
    margin: var(--space-2) 0;
  }

  .content :global(li > p) {
    margin: var(--space-2) 0;
  }

  .content :global(a) {
    color: var(--accent-ink);
    text-underline-offset: 0.18em;
  }

  .content :global(a:hover) {
    text-decoration-thickness: 2px;
  }

  .content :global(code) {
    font-size: 0.92em;
    padding: 0.14em 0.38em;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--surface-soft) 70%, transparent);
    border: 1px solid color-mix(in srgb, var(--line) 62%, transparent);
  }

  .content :global(pre) {
    margin: var(--space-4) 0;
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-soft);
    border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
    overflow-x: auto;
  }

  .content :global(pre code) {
    padding: 0;
    border: none;
    background: transparent;
    font-size: 0.9em;
    line-height: 1.55;
  }

  .content :global(blockquote) {
    margin: var(--space-4) 0;
    padding: var(--space-3) var(--space-4);
    border-left: 4px solid color-mix(in srgb, var(--accent) 42%, var(--line));
    background: color-mix(in srgb, var(--accent-soft) 55%, transparent);
    border-radius: var(--radius-md);
  }

  .content :global(blockquote > :first-child) {
    margin-top: 0;
  }

  .content :global(blockquote > :last-child) {
    margin-bottom: 0;
  }

  .content :global(hr) {
    border: none;
    border-top: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
    margin: var(--space-5) 0;
  }

  .content :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: var(--space-4) 0;
    overflow: hidden;
    border-radius: var(--radius-md);
    border: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
  }

  .content :global(th),
  .content :global(td) {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid color-mix(in srgb, var(--line) 70%, transparent);
    vertical-align: top;
  }

  .content :global(th) {
    text-align: left;
    background: color-mix(in srgb, var(--surface-soft) 65%, transparent);
    font-weight: 600;
  }

  .content :global(tr:last-child td) {
    border-bottom: none;
  }

  /* KaTeX: keep display math readable on narrow screens. */
  .content :global(.katex-display) {
    margin: var(--space-4) 0;
    overflow-x: auto;
    overflow-y: hidden;
    padding: var(--space-2) 0;
  }

  .content :global(.katex-display > .katex) {
    max-width: 100%;
  }

  .content :global(.katex) {
    font-size: 1.02em;
  }

  .content :global(li + li) {
    margin-top: var(--space-2);
  }
</style>
