<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { dev } from '$app/environment';
    import { fade, fly } from 'svelte/transition';
    import { Alert, Badge, Button, SegmentedControl, slideUp } from 'anasthasia';
    import {
        IconBook2,
        IconPlayerPlay,
        IconSearch,
        IconServer,
        IconSettings,
    } from '@tabler/icons-svelte';
    import { commands, events, type RuntimeState, type SearchResult } from '$types/bindings';
    import { formatAppError } from '$lib/errors';
    import { downloadStore as store } from '$lib/download/state.svelte';
    import SourcePicker from '$components/download/SourcePicker.svelte';
    import SearchBar from '$components/download/SearchBar.svelte';
    import ResultGrid from '$components/download/ResultGrid.svelte';
    import DiscoveryServerPanel from '$components/download/DiscoveryServerPanel.svelte';
    import SelectedSeriesPane from '$components/download/SelectedSeriesPane.svelte';

    let runtime = $state<RuntimeState>({ state: 'not_running' });
    let activeTab = $state<'catalog' | 'server'>('catalog');
    let convertAfter = $state(false);
    let detailError = $state('');
    let loadMoreSentinel = $state<HTMLDivElement>();
    let loadMoreObserver: IntersectionObserver | undefined;
    let statusUnlisten: (() => void) | undefined;

    onMount(async () => {
        const status = await commands.suwayomiStatus();
        if (status.status === 'ok') runtime = status.data;
        if (runtime.state === 'ready') await loadSources();
        statusUnlisten = await events.suwayomiStateChangedEvent.listen(async (event) => {
            runtime = event.payload.state;
            if (runtime.state === 'ready') await loadSources();
        });
    });

    onDestroy(() => {
        loadMoreObserver?.disconnect();
        statusUnlisten?.();
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
            store.error = formatAppError(result.error);
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
        } else store.error = formatAppError(result.error);
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
            store.error = formatAppError(result.error);
        }
        store.loadingMore = false;
    }

    async function pickSeries(series: SearchResult) {
        store.selectedSeries = series;
        store.loadingChapters = true;
        detailError = '';
        const result = await commands.listChapters(series.id);
        if (result.status === 'ok') store.chapters = result.data;
        else detailError = formatAppError(result.error);
        store.loadingChapters = false;
    }

    function closeDetail() {
        detailError = '';
        store.resetSelection();
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
            store.error = formatAppError(result.error);
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
        store.selectedChapterIds.clear();
        for (const chapter of store.chapters) {
            store.selectedChapterIds.add(chapter.id);
        }
    }

    function clearChapterSelection() {
        store.selectedChapterIds.clear();
    }

    async function startServer() {
        const result = await commands.suwayomiStart();
        if (result.status === 'error') store.error = formatAppError(result.error);
    }

    function statusVariant(
        state: RuntimeState['state']
    ): 'default' | 'success' | 'warning' | 'danger' {
        if (state === 'ready') return 'success';
        if (state === 'starting') return 'warning';
        if (state === 'error') return 'danger';
        return 'default';
    }

    function statusLabel(state: RuntimeState) {
        if (state.state === 'ready') return dev ? `Ready:${state.port}` : 'Ready';
        if (state.state === 'starting') return 'Starting';
        if (state.state === 'not_installed') return 'Setup needed';
        if (state.state === 'error') return 'Error';
        return 'Stopped';
    }
</script>

<div class="flex h-full flex-col">
    <div
        class="flex flex-shrink-0 items-center justify-between gap-4 border-b border-anasthasia-border px-6 py-4"
    >
        <div class="min-w-0">
            <div class="flex items-center gap-2">
                <h1 class="text-xl font-bold">Discover</h1>
                <Badge variant={statusVariant(runtime.state)}>{statusLabel(runtime)}</Badge>
            </div>
            <p class="mt-0.5 text-sm text-anasthasia-muted">
                Catalog search, extensions, downloads, and Suwayomi runtime
            </p>
        </div>
        <SegmentedControl
            options={[
                { value: 'catalog', label: 'Catalog' },
                { value: 'server', label: 'Suwayomi' },
            ]}
            bind:value={activeTab}
        />
    </div>

    <div class="flex min-h-0 flex-1 overflow-hidden">
        {#if activeTab === 'server'}
            <div class="flex-1 overflow-y-auto px-6 py-5" in:slideUp>
                <div class="w-full">
                    <DiscoveryServerPanel />
                </div>
            </div>
        {:else}
            <div class="flex min-w-0 flex-1 flex-col overflow-y-auto px-6 py-5" in:slideUp>
                {#if runtime.state !== 'ready'}
                    <div
                        class="flex min-h-96 flex-col items-center justify-center gap-4 rounded-xl border border-anasthasia-border bg-anasthasia-surface px-6 text-center"
                    >
                        <div
                            class="flex h-12 w-12 items-center justify-center rounded-xl border border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted"
                        >
                            {#if runtime.state === 'not_installed'}
                                <IconServer size={22} />
                            {:else}
                                <IconSearch size={22} />
                            {/if}
                        </div>
                        <div>
                            <h2 class="text-lg font-bold">Discovery is not ready</h2>
                            <p class="mt-1 max-w-md text-sm text-anasthasia-muted">
                                Install and start Suwayomi-Server here, then search source catalogs
                                without leaving Discover.
                            </p>
                        </div>
                        <div class="flex flex-wrap justify-center gap-2">
                            {#if runtime.state !== 'not_installed'}
                                <Button variant="primary" onclick={startServer}>
                                    <IconPlayerPlay size={15} /> Start server
                                </Button>
                            {/if}
                            <Button variant="secondary" onclick={() => (activeTab = 'server')}>
                                <IconSettings size={15} /> Server setup
                            </Button>
                        </div>
                    </div>
                {:else}
                    <div class="mx-auto flex w-full max-w-[100rem] flex-col gap-4">
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
                    </div>
                {/if}
            </div>

            <aside
                class="hidden w-[34rem] flex-shrink-0 overflow-y-auto border-l border-anasthasia-border bg-anasthasia-bg/70 p-4 2xl:block"
            >
                {#if store.selectedSeries}
                    <div class="h-full" in:slideUp>
                        <SelectedSeriesPane
                            series={store.selectedSeries}
                            chapters={store.chapters}
                            selectedIds={store.selectedChapterIds}
                            loadingChapters={store.loadingChapters}
                            downloading={store.downloading}
                            {convertAfter}
                            {detailError}
                            onToggle={(id) => store.toggleChapter(id)}
                            onSelectAll={selectAllChapters}
                            onClearSelection={clearChapterSelection}
                            onConvertAfter={(value) => (convertAfter = value)}
                            onDownload={download}
                            onClose={closeDetail}
                        />
                    </div>
                {:else}
                    <div
                        class="flex min-h-full flex-col items-center justify-center gap-3 text-center text-sm text-anasthasia-muted"
                    >
                        <div
                            class="flex h-11 w-11 items-center justify-center rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                        >
                            <IconBook2 size={20} />
                        </div>
                        <div>Select a title to inspect chapters.</div>
                    </div>
                {/if}
            </aside>
        {/if}
    </div>

    {#if activeTab === 'catalog' && runtime.state === 'ready' && store.selectedSeries}
        <div
            class="fixed inset-0 z-[90] 2xl:hidden"
            role="presentation"
            transition:fade={{ duration: 120 }}
        >
            <button
                type="button"
                class="absolute inset-0 bg-black/45 backdrop-blur-sm"
                aria-label="Close selected manga"
                onclick={closeDetail}
            ></button>
            <div
                class="absolute inset-x-0 bottom-0 max-h-[calc(100vh-4rem)] overflow-hidden rounded-t-2xl border border-anasthasia-border bg-anasthasia-bg p-3 shadow-2xl sm:inset-y-10 sm:right-6 sm:left-auto sm:max-h-none sm:w-[min(44rem,calc(100vw-8rem))] sm:rounded-xl"
                role="dialog"
                aria-modal="true"
                aria-label="Selected manga"
                tabindex="-1"
                in:fly={{ y: 18, duration: 180 }}
                out:fly={{ y: 18, duration: 140 }}
            >
                <SelectedSeriesPane
                    series={store.selectedSeries}
                    chapters={store.chapters}
                    selectedIds={store.selectedChapterIds}
                    loadingChapters={store.loadingChapters}
                    downloading={store.downloading}
                    {convertAfter}
                    {detailError}
                    onToggle={(id) => store.toggleChapter(id)}
                    onSelectAll={selectAllChapters}
                    onClearSelection={clearChapterSelection}
                    onConvertAfter={(value) => (convertAfter = value)}
                    onDownload={download}
                    onClose={closeDetail}
                />
            </div>
        </div>
    {/if}
</div>
