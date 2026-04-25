import { untrack } from 'svelte';

type Scope = {
    keys: [string, string][];
    exclusive: boolean;
};

class KeyHintState {
    private scopes = $state<Record<string, Scope>>({});
    private nextId = 0;

    register(keys: [string, string][], exclusive = false): () => void {
        const id = `kh-${++this.nextId}`;
        untrack(() => {
            this.scopes = { ...this.scopes, [id]: { keys, exclusive } };
        });
        return () =>
            untrack(() => {
                const { [id]: _, ...rest } = this.scopes;
                this.scopes = rest;
            });
    }

    get(): [string, string][] {
        const allScopes = Object.values(this.scopes);
        const hasExclusive = allScopes.some((s) => s.exclusive);
        const merged = new Map<string, string>();
        allScopes.forEach((scope) => {
            if (hasExclusive && !scope.exclusive) return;
            scope.keys.forEach(([key, label]) => merged.set(key, label));
        });
        return Array.from(merged.entries()) as [string, string][];
    }
}

export const keyHint = new KeyHintState();

/** Registers hints for the entire lifetime of the element (mount → destroy). */
export function mountedHint(node: HTMLElement, keys: [string, string][]) {
    let cleanup = keyHint.register(keys);
    return {
        update(newKeys: [string, string][]) {
            cleanup();
            cleanup = keyHint.register(newKeys);
        },
        destroy() {
            cleanup();
        },
    };
}

/** Registers hints only while the element is focused. */
export function handleKeyHint(
    node: HTMLElement,
    data: { keys: [string, string][]; exclusive?: boolean },
) {
    let unregister: (() => void) | null = null;

    const add = () => {
        unregister?.();
        unregister = keyHint.register(data.keys, data.exclusive ?? false);
    };
    const remove = () => {
        unregister?.();
        unregister = null;
    };

    node.addEventListener('focus', add);
    node.addEventListener('blur', remove);
    if (document.activeElement === node) add();

    return {
        update(newData: typeof data) {
            remove();
            data = newData;
            if (document.activeElement === node) add();
        },
        destroy() {
            remove();
            node.removeEventListener('focus', add);
            node.removeEventListener('blur', remove);
        },
    };
}
