const KEY = 'thasia:settings';

class UiPrefs {
    showKeyHints = $state(true);

    init() {
        try {
            const d = JSON.parse(localStorage.getItem(KEY) ?? '{}');
            if (d.showKeyHints !== undefined) this.showKeyHints = d.showKeyHints;
        } catch {}
    }
}

export const uiPrefs = new UiPrefs();
