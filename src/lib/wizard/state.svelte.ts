import type { VolumeMeta } from '$types/bindings';
import { SvelteSet } from 'svelte/reactivity';

export type VolumeEdit = {
    volumeNum: number;
    pages: Array<{
        originalPageIndex: number | null;
        sourceVolumeNum: number | null;
        customPath: string | null;
        excluded: boolean;
    }>;
};

export class WizardStore {
    // Step 1 — Source
    sourcePath = $state('');

    // Step 2 — Destination
    outputDir = $state('');
    outputName = $state('output');
    createDirectory = $state(false);

    // Step 3 — Image Format
    imageFormat = $state<'avif' | 'webp' | 'original'>('avif');
    maxWidth = $state<number | null>(null);
    forceReencode = $state(false);
    cleanTones = $state(false);

    // Step 4 — Container
    container = $state<'cbz' | 'epub' | 'raw'>('cbz');

    // Step 5 — Direction (epub only)
    direction = $state<'ltr' | 'rtl'>('ltr');

    // Step 6 — Bundling
    bundle = $state<'auto' | 'flatten'>('auto');
    volumeSeparator = $state(' - ');
    hideSingleVolume = $state(false);

    // Step 7 — Page editor result
    scanResult = $state<VolumeMeta[] | null>(null);
    pageEdits = $state<VolumeEdit[]>([]);

    // Navigation
    currentStepId = $state('source');
    completedStepIds = new SvelteSet<string>();

    /** True while Step9 is actively converting — locks all navigation away. */
    converting = $state(false);

    markComplete(id: string) {
        this.completedStepIds.add(id);
    }

    /** Reshape pageEdits to match the current bundle mode + scanResult.
     * - auto: re-explode pageEdits per scan volume
     * - flatten: merge all pages into a single output volume
     */
    reconcileBundling(): void {
        if (this.bundle === 'flatten' && this.pageEdits.length > 1) {
            const firstNum = this.pageEdits[0]?.volumeNum ?? 1;
            this.pageEdits = [
                {
                    volumeNum: firstNum,
                    pages: this.pageEdits.flatMap((ve) => ve.pages),
                },
            ];
        } else if (
            this.bundle === 'auto' &&
            this.scanResult &&
            this.pageEdits.length === 1 &&
            this.scanResult.length > 1
        ) {
            this.pageEdits = this.scanResult.map((vol) => ({
                volumeNum: vol.volume_num,
                pages: vol.pages.map((_, i) => ({
                    originalPageIndex: i,
                    sourceVolumeNum: vol.volume_num,
                    customPath: null,
                    excluded: false,
                })),
            }));
        }
    }

    reset() {
        this.sourcePath = '';
        this.outputDir = '';
        this.outputName = 'output';
        this.createDirectory = false;
        this.imageFormat = 'avif';
        this.maxWidth = null;
        this.forceReencode = false;
        this.cleanTones = false;
        this.container = 'cbz';
        this.direction = 'ltr';
        this.bundle = 'auto';
        this.volumeSeparator = ' - ';
        this.hideSingleVolume = false;
        this.scanResult = null;
        this.pageEdits = [];
        this.currentStepId = 'source';
        this.completedStepIds.clear();
        this.converting = false;
    }
}

export const wizard = new WizardStore();
