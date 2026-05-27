<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { Alert, Button, Dialog } from 'anasthasia';
    import { IconSettings, IconSearch } from '@tabler/icons-svelte';
    import { commands, type RuntimeState, type SearchResult } from '$types/bindings';
    import { downloadStore as store } from '$lib/download/state.svelte';
    import SourcePicker from '$components/download/SourcePicker.svelte';
    import SearchBar from '$components/download/SearchBar.svelte';
    import ResultGrid from '$components/download/ResultGrid.svelte';
    import SeriesDetail from '$components/download/SeriesDetail.svelte';
    import DownloadProgress from '$components/download/DownloadProgress.svelte';

    let runtime = $state<RuntimeState>({ state: 'not_running' });
    let convertAfter = $state(false);
    let detailError = $state('');
    let loadMoreSentinel = $state<HTMLDivElement>();
    let loadMoreObserver: IntersectionObserver | undefined;

    onMount(async () => {
        const status = await commands.suwayomiStatus();
        if (status.status === 'ok') runtime = status.data;
        if (runtime.state === 'ready') await loadSources();
    });

    onDestroy(() => {
        loadMoreObserver?.disconnect();
    });

    $effect(() => {
        loadMoreObserver?.disconnect();
        if (!loadMoreSentinel) return;

        loadMoreObserver = new IntersectionObserver(
            ([entry]) => {
                if (entry?.isIntersecting) void loadMore();
            },
            { rootMargin: '420px 0px' }
        );
        loadMoreObserver.observe(loadMoreSentinel);

        return () => loadMoreObserver?.disconnect();
    });

    async function loadSources() {
        store.error = '';
        const result = await commands.listInstalledSources();
        if (result.status === 'ok') {
            store.sources = result.data.filter((source) => {
                const name = source.name.toLowerCase();
                const lang = source.lang?.toLowerCase();
                return name !== 'local source' && lang !== 'localsourcelang';
            });
            if (!store.sources.some((source) => source.id === store.selectedSourceId)) {
                store.selectedSourceId = store.sources[0]?.id ?? '';
            }
        } else {
            store.error = result.error;
        }
    }

    async function search() {
        if (!store.selectedSourceId || !store.query.trim()) return;
        store.searching = true;
        store.error = '';
        detailError = '';
        store.resetResults();
        const result = await commands.searchSource(store.selectedSourceId, store.query, 1);
        if (result.status === 'ok') {
            store.results = result.data.results;
            store.page = 1;
            store.hasNextPage = result.data.has_next_page;
        } else store.error = result.error;
        store.searching = false;
    }

    async function loadMore() {
        if (
            !store.selectedSourceId ||
            !store.query.trim() ||
            !store.hasNextPage ||
            store.searching ||
            store.loadingMore
        )
            return;
        store.loadingMore = true;
        store.error = '';
        const nextPage = store.page + 1;
        const result = await commands.searchSource(store.selectedSourceId, store.query, nextPage);
        if (result.status === 'ok') {
            const existing = new Set(store.results.map((item) => item.id));
            store.results = [
                ...store.results,
                ...result.data.results.filter((item) => !existing.has(item.id)),
            ];
            store.page = nextPage;
            store.hasNextPage = result.data.has_next_page;
        } else {
            store.error = result.error;
        }
        store.loadingMore = false;
    }

    async function pickSeries(series: SearchResult) {
        store.selectedSeries = series;
        store.loadingChapters = true;
        detailError = '';
        const result = await commands.listChapters(series.id);
        if (result.status === 'ok') store.chapters = result.data;
        else detailError = result.error;
        store.loadingChapters = false;
    }

    async function download() {
        if (!store.selectedSeries) return;
        store.downloading = true;
        store.error = '';
        // Pass the full chapter objects sorted by chapter number so the backend
        // has volume/chapter metadata for Hakuneko-style directory naming.
        const selectedChapters = store.chapters
            .filter((ch) => store.selectedChapterIds.has(ch.id))
            .sort((a, b) => a.chapter_number - b.chapter_number);
        const result = await commands.downloadSeries(
            store.selectedSeries.id,
            selectedChapters,
            convertAfter
        );
        if (result.status === 'error') {
            store.error = result.error;
        } else if (convertAfter) {
            const title = store.selectedSeries.title;
            detailError = '';
            store.resetSelection();
            store.downloading = false;
            await goto(`/convert?source=discovery&name=${encodeURIComponent(title)}`);
            return;
        }
        store.downloading = false;
    }

    function selectAllChapters() {
        store.selectedChapterIds = new Set(store.chapters.map((chapter) => chapter.id));
    }

    function clearChapterSelection() {
        store.selectedChapterIds = new Set();
    }
</script>

<div class="flex h-full flex-col">
    <div
        class="flex flex-shrink-0 items-baseline justify-between gap-4 border-b border-anasthasia-border px-8 py-5"
    >
        <div>
            <h1 class="text-xl font-bold">Discover</h1>
            <p class="mt-0.5 text-sm text-anasthasia-muted">
                Search installed Suwayomi sources and download chapters
            </p>
        </div>
    </div>

    <div class="flex flex-1 flex-col overflow-y-auto">
        <div class="mx-auto flex w-full max-w-6xl flex-col gap-4 px-8 py-6">
            {#if runtime.state !== 'ready'}
                <div
                    class="flex min-h-80 flex-col items-center justify-center gap-4 rounded-xl border border-anasthasia-border bg-anasthasia-surface px-6 text-center"
                >
                    <div
                        class="flex h-12 w-12 items-center justify-center rounded-xl border border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted"
                    >
                        <IconSearch size={22} />
                    </div>
                    <div>
                        <h2 class="text-lg font-bold">Discovery is not ready</h2>
                        <p class="mt-1 max-w-md text-sm text-anasthasia-muted">
                            Install and start Suwayomi-Server from Settings before opening the
                            catalog.
                        </p>
                    </div>
                    <Button variant="primary" onclick={() => goto('/settings/discovery')}>
                        <IconSettings size={15} /> Discovery settings
                    </Button>
                </div>
            {:else}
                <div
                    class="flex flex-col gap-4 rounded-xl border border-anasthasia-border bg-anasthasia-surface px-4 py-4"
                >
                    <SourcePicker
                        sources={store.sources}
                        selectedSourceId={store.selectedSourceId}
                        onSelect={(id) => {
                            store.selectedSourceId = id;
                            store.resetResults();
                        }}
                        onRefresh={loadSources}
                    />
                    <SearchBar
                        query={store.query}
                        disabled={!store.selectedSourceId}
                        searching={store.searching}
                        onQuery={(value) => (store.query = value)}
                        onSearch={search}
                    />
                </div>

                {#if store.error}
                    <Alert variant="danger" title="Discovery failed">{store.error}</Alert>
                {/if}

                {#if store.results.length > 0}
                    <ResultGrid results={store.results} onPick={pickSeries} />
                    <div
                        bind:this={loadMoreSentinel}
                        class="rounded-xl border border-anasthasia-border bg-anasthasia-surface px-4 py-3 text-center text-sm text-anasthasia-muted"
                    >
                        {#if store.loadingMore}
                            Loading more results…
                        {:else if store.hasNextPage}
                            Showing {store.results.length} result{store.results.length === 1
                                ? ''
                                : 's'} · page {store.page}
                        {:else}
                            No more results
                        {/if}
                    </div>
                {:else}
                    <div
                        class="rounded-xl border border-dashed border-anasthasia-border px-4 py-10 text-center text-sm text-anasthasia-muted"
                    >
                        No results yet.
                    </div>
                {/if}

                <Dialog
                    open={!!store.selectedSeries}
                    title="Series details"
                    class="!max-w-[min(90vw,68rem)]"
                    onclose={() => {
                        detailError = '';
                        store.resetSelection();
                    }}
                >
                    {#if store.selectedSeries}
                        {#if store.loadingChapters}
                            <div
                                class="flex min-h-80 items-center justify-center text-sm text-anasthasia-muted"
                            >
                                Loading chapters…
                            </div>
                        {:else if detailError}
                            <div class="flex min-h-80 flex-col justify-center gap-4">
                                <SeriesDetail
                                    series={store.selectedSeries}
                                    chapters={[]}
                                    selectedIds={store.selectedChapterIds}
                                    downloading={store.downloading}
                                    {convertAfter}
                                    onToggle={(id) => store.toggleChapter(id)}
                                    onSelectAll={selectAllChapters}
                                    onClearSelection={clearChapterSelection}
                                    onConvertAfter={(value) => (convertAfter = value)}
                                    onDownload={download}
                                />
                                <Alert variant="danger" title="Could not load chapters"
                                    >{detailError}</Alert
                                >
                            </div>
                        {:else}
                            <SeriesDetail
                                series={store.selectedSeries}
                                chapters={store.chapters}
                                selectedIds={store.selectedChapterIds}
                                downloading={store.downloading}
                                {convertAfter}
                                onToggle={(id) => store.toggleChapter(id)}
                                onSelectAll={selectAllChapters}
                                onClearSelection={clearChapterSelection}
                                onConvertAfter={(value) => (convertAfter = value)}
                                onDownload={download}
                            />
                            <DownloadProgress />
                        {/if}
                    {/if}
                </Dialog>
            {/if}
        </div>
    </div>
</div>
