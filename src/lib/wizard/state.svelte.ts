import type { VolumeMeta } from '$types/bindings';

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
  completedStepIds = $state(new Set<string>());

  markComplete(id: string) {
    this.completedStepIds = new Set([...this.completedStepIds, id]);
  }

  reset() {
    this.sourcePath = '';
    this.outputDir = '';
    this.outputName = 'output';
    this.createDirectory = false;
    this.imageFormat = 'avif';
    this.maxWidth = null;
    this.container = 'cbz';
    this.direction = 'ltr';
    this.bundle = 'auto';
    this.volumeSeparator = ' - ';
    this.hideSingleVolume = false;
    this.scanResult = null;
    this.pageEdits = [];
    this.currentStepId = 'source';
    this.completedStepIds = new Set();
  }
}

export const wizard = new WizardStore();
