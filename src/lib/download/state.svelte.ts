import type { ChapterMeta, SearchResult, SourceInfo } from '$types/bindings';

export class DownloadStore {
    sources = $state<SourceInfo[]>([]);
    selectedSourceId = $state('');
    query = $state('');
    results = $state<SearchResult[]>([]);
    selectedSeries = $state<SearchResult | null>(null);
    chapters = $state<ChapterMeta[]>([]);
    selectedChapterIds = $state<Set<number>>(new Set());
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
        this.selectedChapterIds = new Set();
    }

    resetResults() {
        this.results = [];
        this.page = 1;
        this.hasNextPage = false;
        this.resetSelection();
    }

    toggleChapter(id: number) {
        const next = new Set(this.selectedChapterIds);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        this.selectedChapterIds = next;
    }
}

export const downloadStore = new DownloadStore();
