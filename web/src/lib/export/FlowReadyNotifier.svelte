<script lang="ts">
  import { useNodesInitialized, useViewportInitialized } from "@xyflow/svelte";

  interface Props {
    onReady: () => void;
  }

  let { onReady }: Props = $props();

  const nodesInitialized = useNodesInitialized();
  const viewportInitialized = useViewportInitialized();

  let didNotify = $state(false);

  const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

  $effect(() => {
    if (didNotify) {
      return;
    }

    if (nodesInitialized.current && viewportInitialized.current) {
      didNotify = true;
      void (async () => {
        await nextFrame();
        await nextFrame();
        onReady();
      })();
    }
  });
</script>
