type KeyHandler = {
    id: string;
    callback: (event: KeyboardEvent) => void | boolean;
};

function normalizeCombo(combo: string): string {
    return combo.toLowerCase();
}

class KeyboardManager {
    private handlers: Map<string, KeyHandler[]> = new Map();

    constructor() {
        this.handleKeyDown = this.handleKeyDown.bind(this);
    }

    register(
        combo: string,
        callback: (event: KeyboardEvent) => void | boolean,
        id?: string,
    ): string {
        combo = normalizeCombo(combo);
        if (!this.handlers.has(combo)) this.handlers.set(combo, []);
        id = id ?? this.generateId();
        const handlers = this.handlers.get(combo)!;
        if (handlers.find((h) => h.id === id)) throw new Error(`Handler "${id}" already registered`);
        handlers.push({ id, callback });
        return id;
    }

    unregister(id: string): void {
        for (const [key, handlers] of this.handlers.entries()) {
            const idx = handlers.findIndex((h) => h.id === id);
            if (idx !== -1) {
                if (handlers.length === 1) this.handlers.delete(key);
                else handlers.splice(idx, 1);
                return;
            }
        }
    }

    smartRegister(handlers: [string, (event: KeyboardEvent) => void | boolean, string?][]): () => void {
        const ids = handlers.map((args) => this.register(...args));
        return () => ids.forEach((id) => this.unregister(id));
    }

    mount(): () => void {
        window.addEventListener('keydown', this.handleKeyDown);
        return () => window.removeEventListener('keydown', this.handleKeyDown);
    }

    private handleKeyDown(event: KeyboardEvent): void {
        const modifiers = [
            event.ctrlKey ? 'ctrl' : '',
            event.altKey ? 'alt' : '',
            event.shiftKey ? 'shift' : '',
            event.metaKey ? 'meta' : '',
        ].filter(Boolean);

        const combo = normalizeCombo(
            modifiers.length > 0 ? `${modifiers.join('+')}+${event.code}` : event.code,
        );

        const handlers = this.handlers.get(combo) ?? [];

        const isInInput =
            event.target instanceof HTMLInputElement ||
            event.target instanceof HTMLTextAreaElement ||
            (event.target instanceof HTMLElement && event.target.isContentEditable);

        const isLetterCombo = /^key[a-z]$/.test(event.code);
        const isAltArrow = event.altKey && event.code.startsWith('Arrow');

        for (let i = handlers.length - 1; i >= 0; i--) {
            if (isInInput && (isLetterCombo || isAltArrow)) continue;
            if (handlers[i].callback(event) === true) break;
        }
    }

    private nextId = 0;
    private generateId(): string {
        return `kb-${++this.nextId}`;
    }
}

export const keyboard = new KeyboardManager();
