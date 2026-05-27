<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { ProgressBar } from 'anasthasia';
    import { events, type ChapterDownloadPhase } from '$types/bindings';

    let current = $state(0);
    let total = $state(0);
    let label = $state('');
    let complete = $state('');
    let error = $state('');
    let phase = $state<ChapterDownloadPhase | null>(null);
    let tick = $state(0);
    let unlisteners: Array<() => void> = [];

    let waitingValue = $derived(0.08 + ((tick % 18) / 18) * 0.22);
    let progressValue = $derived(
        complete
            ? 1
            : total
              ? current > 0
                  ? current / total
                  : phase === 'downloading'
                    ? waitingValue
                    : 0
              : 0
    );

    onMount(async () => {
        unlisteners.push(
            await events.downloadStartEvent.listen((event) => {
                total = event.payload.total_chapters;
                current = 0;
                label = event.payload.series_title;
                complete = '';
                error = '';
                phase = 'downloading';
                tick = 0;
            }),
            await events.chapterDownloadEvent.listen((event) => {
                current = event.payload.current;
                total = event.payload.total;
                label = event.payload.current_chapter;
                phase = event.payload.phase;
                tick = event.payload.tick;
            }),
            await events.downloadCompleteEvent.listen((event) => {
                if (event.payload.success)
                    complete = event.payload.output_dir || 'Download complete';
                else error = event.payload.error || 'Download failed';
            })
        );
    });

    onDestroy(() => unlisteners.forEach((unlisten) => unlisten()));
</script>

{#if total > 0 || complete || error}
    <div class="rounded-xl border border-anasthasia-border bg-anasthasia-surface px-4 py-3">
        <div class="mb-2 flex justify-between gap-4 text-xs text-anasthasia-muted">
            <span class="truncate">{error || complete || label}</span>
            {#if total > 0}
                <span>{current}/{total}</span>
            {/if}
        </div>
        <ProgressBar
            value={progressValue}
            variant={error ? 'danger' : complete ? 'success' : 'accent'}
            class="h-1.5"
        />
    </div>
{/if}
