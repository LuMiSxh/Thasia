import { wizard, type ColorEnhanceMode, type SharpenMode } from '$lib/wizard/state.svelte';
import { uiPrefs } from 'anasthasia';

const STORAGE_KEY = 'thasia:settings';

export type Settings = {
    imageFormat: 'avif' | 'webp' | 'original';
    container: 'cbz' | 'epub' | 'raw';
    direction: 'ltr' | 'rtl';
    bundle: 'auto' | 'flatten';
    volumeSeparator: string;
    hideSingleVolume: boolean;
    createDirectory: boolean;
    maxWidth: number | null;
    forceReencode: boolean;
    cleanTones: boolean;
    colorEnhance: ColorEnhanceMode;
    sharpen: SharpenMode;
    showKeyHints: boolean;
    /** Pre-filled in the wizard setup step. Empty = no default. */
    defaultOutputDir: string;
};

export const DEFAULT_SETTINGS: Settings = {
    imageFormat: 'avif',
    container: 'cbz',
    direction: 'ltr',
    bundle: 'auto',
    volumeSeparator: ' - ',
    hideSingleVolume: false,
    createDirectory: false,
    maxWidth: null,
    forceReencode: false,
    cleanTones: false,
    colorEnhance: 'off',
    sharpen: 'off',
    showKeyHints: true,
    defaultOutputDir: '',
};

export function loadSettings(): Settings {
    if (typeof localStorage === 'undefined') return { ...DEFAULT_SETTINGS };
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    try {
        return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
    } catch {
        return { ...DEFAULT_SETTINGS };
    }
}

export function saveSettings(settings: Settings): void {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
}

/** Copy persisted settings into the wizard store (skips fields the user already touched). */
export function applyToWizard(settings: Settings, opts: { force?: boolean } = {}): void {
    const force = opts.force ?? false;
    if (force || !wizard.sourcePath) {
        wizard.imageFormat = settings.imageFormat;
        wizard.container = settings.container;
        wizard.direction = settings.direction;
        wizard.bundle = settings.bundle;
        wizard.volumeSeparator = settings.volumeSeparator;
        wizard.hideSingleVolume = settings.hideSingleVolume;
        wizard.createDirectory = settings.createDirectory;
        wizard.maxWidth = settings.maxWidth;
        wizard.forceReencode = settings.forceReencode;
        wizard.cleanTones = settings.cleanTones;
        wizard.colorEnhance = settings.colorEnhance;
        wizard.sharpen = settings.sharpen;
    }
    // Output dir: only set if empty, regardless of force (don't clobber user's manual pick).
    if (!wizard.outputDir && settings.defaultOutputDir) {
        wizard.outputDir = settings.defaultOutputDir;
    }
}

/** Apply just the UI prefs (keyhint bar). Safe to call on every page. */
export function applyUiPrefs(settings: Settings): void {
    uiPrefs.showKeyHints = settings.showKeyHints;
}
