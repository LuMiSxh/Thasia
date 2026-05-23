import { keyHint } from 'anasthasia';

export { keyHint };

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
    data: { keys: [string, string][]; exclusive?: boolean }
) {
    let unregister: (() => void) | null = null;

    const add = () => {
        unregister?.();
        unregister = keyHint.register(data.keys);
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
