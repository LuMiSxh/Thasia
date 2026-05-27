import { commands, type DiscoverySettings } from '$types/bindings';

export const DEFAULT_DISCOVERY_SETTINGS: DiscoverySettings = {
    enabled: false,
    installedVersion: null,
    autoStart: false,
    lastUpdateCheck: null,
    downloadDir: null,
    extensionRepos: ['https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json'],
};

export async function loadDiscoverySettings(): Promise<DiscoverySettings> {
    const result = await commands.getDiscoverySettings();
    if (result.status === 'error') return { ...DEFAULT_DISCOVERY_SETTINGS };
    return { ...DEFAULT_DISCOVERY_SETTINGS, ...result.data };
}

export async function saveDiscoverySettings(settings: DiscoverySettings): Promise<void> {
    const result = await commands.setDiscoverySettings(settings);
    if (result.status === 'error') throw new Error(result.error);
    if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('thasia:discovery-settings-changed'));
    }
}

export function canShowDiscover(settings: DiscoverySettings): boolean {
    return settings.enabled && settings.installedVersion !== null;
}
