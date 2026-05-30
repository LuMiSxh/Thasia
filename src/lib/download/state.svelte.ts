import type { ChapterMeta, SearchResult, SourceInfo } from '$types/bindings';
import { SvelteSet } from 'svelte/reactivity';

export class DownloadStore {
    sources = $state<SourceInfo[]>([]);
    selectedSourceId = $state('');
    query = $state('');
    results = $state<SearchResult[]>([]);
    selectedSeries = $state<SearchResult | null>(null);
    chapters = $state<ChapterMeta[]>([]);
    selectedChapterIds = new SvelteSet<number>();
    searching = $state(false);
    loadingMore = $state(false);
    page = $state(1);
    hasNextPage = $state(false);
    loadingChapters = $state(false);
    downloading = $state(false);
    error = $state('');

    resetSelection() {
        this.selectedSeries = null;
        this.chapters = [];
        this.selectedChapterIds.clear();
    }

    resetResults() {
        this.results = [];
        this.page = 1;
        this.hasNextPage = false;
        this.resetSelection();
    }

    toggleChapter(id: number) {
        if (this.selectedChapterIds.has(id)) this.selectedChapterIds.delete(id);
        else this.selectedChapterIds.add(id);
    }
}

export const downloadStore = new DownloadStore();
