# Keyboard Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a complete keyboard navigation layer to Thasia — a `KeyboardManager` for shortcut routing, a scoped `KeyHintState` for contextual hints, a toggleable hint bar, and per-step bindings throughout the wizard.

**Architecture:** `keyboard.ts` is a singleton manager that listens on `window` and dispatches to registered handlers. `keyhint.svelte.ts` holds reactive scoped hint state and exports two Svelte actions (`mountedHint` for always-on hints tied to element lifecycle, `handleKeyHint` for focus-driven hints). `KeyHintBar.svelte` reads hint state reactively and renders an animated strip at the bottom of `<main>`. No component ever calls `keyHint.register()` manually — everything goes through actions.

**Tech Stack:** Svelte 5 runes, TypeScript, Tailwind CSS, `@tauri-apps/api/core` (already installed), `navigator.platform` for OS detection (no extra plugin needed).

---

## File Map

| File | Status | Responsibility |
|---|---|---|
| `src/lib/keyboard.ts` | **Create** | KeyboardManager singleton — event routing, combo matching |
| `src/lib/keyhint.svelte.ts` | **Create** | KeyHintState singleton + `mountedHint` + `handleKeyHint` actions |
| `src/lib/ui-prefs.svelte.ts` | **Create** | Reactive `showKeyHints` preference, loaded from localStorage |
| `src/components/ui/KeyComboDisplay.svelte` | **Create** | OS-aware combo string → styled Kbd badges |
| `src/components/ui/KeyHintBar.svelte` | **Create** | Animated hint strip; reads `keyHint.get()` reactively |
| `src/components/ui/index.ts` | **Modify** | Export the two new UI components |
| `src/routes/+layout.svelte` | **Modify** | Mount keyboard, render hint bar, register global shortcuts |
| `src/routes/convert/+page.svelte` | **Modify** | Register Alt+→/← wizard nav shortcuts |
| `src/routes/settings/+page.svelte` | **Modify** | Add `showKeyHints` toggle + Interface panel |
| `src/components/wizard/Step3ImageFormat.svelte` | **Modify** | A/W/O shortcuts |
| `src/components/wizard/Step4Container.svelte` | **Modify** | C/E/R shortcuts |
| `src/components/wizard/Step5Direction.svelte` | **Modify** | L/R shortcuts |
| `src/components/wizard/Step6Bundling.svelte` | **Modify** | A/F shortcuts |
| `src/components/wizard/Step7PageEditor.svelte` | **Modify** | ←/→ volume navigation |
| `src/components/wizard/Step9Convert.svelte` | **Modify** | Enter to start over when done |

---

## Task 1: KeyboardManager

**Files:**
- Create: `src/lib/keyboard.ts`

- [ ] **Step 1: Create `src/lib/keyboard.ts`**

```ts
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

        // Skip bare letter keys and Alt+Arrow in text inputs
        const isLetterCombo = /^key[a-z]$/.test(event.code);
        const isAltArrow = event.altKey && event.code.startsWith('Arrow');

        for (let i = handlers.length - 1; i >= 0; i--) {
            if (isInInput && (isLetterCombo || isAltArrow)) continue;
            if (handlers[i].callback(event) === true) break;
        }
    }

    private generateId(): string {
        let id: string;
        do {
            id = Math.random().toString(36).slice(2, 9);
        } while ([...this.handlers.values()].flat().some((h) => h.id === id));
        return id;
    }
}

export const keyboard = new KeyboardManager();
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/Luca/Documents/Projekte/Thasia && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors related to `keyboard.ts`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/keyboard.ts
git commit -m "feat: add KeyboardManager singleton"
```

---

## Task 2: KeyHintState + Svelte actions

**Files:**
- Create: `src/lib/keyhint.svelte.ts`

- [ ] **Step 1: Create `src/lib/keyhint.svelte.ts`**

```ts
import { untrack } from 'svelte';

type Scope = {
    keys: [string, string][];
    exclusive: boolean;
};

class KeyHintState {
    private scopes = $state<Record<string, Scope>>({});

    register(keys: [string, string][], exclusive = false): () => void {
        const id = Math.random().toString(36).slice(2);
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
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /Users/Luca/Documents/Projekte/Thasia && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors related to `keyhint.svelte.ts`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/keyhint.svelte.ts
git commit -m "feat: add KeyHintState with mountedHint and handleKeyHint actions"
```

---

## Task 3: UI preferences singleton

**Files:**
- Create: `src/lib/ui-prefs.svelte.ts`

- [ ] **Step 1: Create `src/lib/ui-prefs.svelte.ts`**

```ts
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
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/ui-prefs.svelte.ts
git commit -m "feat: add UiPrefs singleton for showKeyHints preference"
```

---

## Task 4: KeyComboDisplay component

**Files:**
- Create: `src/components/ui/KeyComboDisplay.svelte`

- [ ] **Step 1: Create `src/components/ui/KeyComboDisplay.svelte`**

```svelte
<script lang="ts">
    import Kbd from './Kbd.svelte';

    let { combo }: { combo: string } = $props();

    const isMac = typeof navigator !== 'undefined' && navigator.platform.startsWith('Mac');

    const macSymbols: Record<string, string> = {
        meta: '⌘', alt: '⌥', ctrl: '⌃', shift: '⇧',
        arrowright: '→', arrowleft: '←', arrowup: '↑', arrowdown: '↓',
        enter: '↩', escape: 'Esc', backspace: '⌫', tab: '⇥', space: '␣',
    };

    const winSymbols: Record<string, string> = {
        meta: 'Win', alt: 'Alt', ctrl: 'Ctrl', shift: '⇧',
        arrowright: '→', arrowleft: '←', arrowup: '↑', arrowdown: '↓',
        enter: '↵', escape: 'Esc', backspace: '⌫', tab: '⇥', space: '␣',
    };

    function formatPart(part: string): string {
        const symbols = isMac ? macSymbols : winSymbols;
        if (symbols[part]) return symbols[part];
        if (part.startsWith('key')) return part.slice(3).toUpperCase();
        if (part.startsWith('digit')) return part.slice(5);
        return part.toUpperCase();
    }

    let parts = $derived(combo.toLowerCase().split('+').map(formatPart));
</script>

<span class="inline-flex items-center gap-0.5">
    {#each parts as part, i (i)}
        <Kbd>{part}</Kbd>
    {/each}
</span>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ui/KeyComboDisplay.svelte
git commit -m "feat: add KeyComboDisplay component with OS-aware symbols"
```

---

## Task 5: KeyHintBar component

**Files:**
- Create: `src/components/ui/KeyHintBar.svelte`

- [ ] **Step 1: Create `src/components/ui/KeyHintBar.svelte`**

```svelte
<script lang="ts">
    import { keyHint } from '$lib/keyhint.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { cubicOut } from 'svelte/easing';
    import KeyComboDisplay from './KeyComboDisplay.svelte';

    function glassIn(node: Element, { duration = 200 }: { duration?: number } = {}) {
        const style = getComputedStyle(node);
        const w = parseFloat(style.width);
        const mr = parseFloat(style.marginRight);
        return {
            duration,
            css: (t: number) => {
                const e = cubicOut(t);
                return `
                    width:${w * e}px;
                    margin-right:${mr * e}px;
                    opacity:${e};
                    transform:translateX(${(1 - e) * -16}px) scale(${0.9 + 0.1 * e});
                    filter:blur(${(1 - e) * 4}px);
                    overflow:hidden;
                    white-space:nowrap;
                `;
            },
        };
    }

    function glassOut(node: Element, { duration = 200 }: { duration?: number } = {}) {
        const style = getComputedStyle(node);
        const w = parseFloat(style.width);
        const mr = parseFloat(style.marginRight);
        return {
            duration,
            css: (t: number) => {
                const e = cubicOut(t);
                return `
                    width:${w * e}px;
                    margin-right:${mr * e}px;
                    opacity:${e};
                    transform:translateX(${(1 - e) * -16}px) scale(${0.9 + 0.1 * e});
                    filter:blur(${(1 - e) * 4}px);
                    overflow:hidden;
                    white-space:nowrap;
                `;
            },
        };
    }

    let hints = $derived(keyHint.get());
</script>

{#if uiPrefs.showKeyHints && hints.length > 0}
    <div class="flex h-8 flex-shrink-0 items-center gap-0 border-t border-thasia-border bg-thasia-surface px-4 overflow-hidden">
        {#each hints as [combo, label] (combo)}
            <div
                class="mr-5 flex items-center gap-2"
                in:glassIn
                out:glassOut
            >
                <KeyComboDisplay {combo} />
                <span class="text-xs text-thasia-muted whitespace-nowrap">{label}</span>
            </div>
        {/each}
    </div>
{/if}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ui/KeyHintBar.svelte
git commit -m "feat: add KeyHintBar with animated glass transitions"
```

---

## Task 6: Export new UI components

**Files:**
- Modify: `src/components/ui/index.ts`

- [ ] **Step 1: Add exports to `src/components/ui/index.ts`**

Append these two lines to the file:

```ts
export { default as KeyComboDisplay } from './KeyComboDisplay.svelte';
export { default as KeyHintBar } from './KeyHintBar.svelte';
```

The full file should now read:

```ts
export { default as Button } from './Button.svelte';
export { default as Toggle } from './Toggle.svelte';
export { default as Input } from './Input.svelte';
export { default as Select } from './Select.svelte';
export { default as Badge } from './Badge.svelte';
export { default as Card } from './Card.svelte';
export { default as SectionLabel } from './SectionLabel.svelte';
export { default as ProgressBar } from './ProgressBar.svelte';
export { default as Kbd } from './Kbd.svelte';
export { default as SegmentedControl } from './SegmentedControl.svelte';
export { default as KeyComboDisplay } from './KeyComboDisplay.svelte';
export { default as KeyHintBar } from './KeyHintBar.svelte';
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ui/index.ts
git commit -m "feat: export KeyComboDisplay and KeyHintBar from ui index"
```

---

## Task 7: Wire up layout — global shortcuts + hint bar

**Files:**
- Modify: `src/routes/+layout.svelte`

- [ ] **Step 1: Replace `src/routes/+layout.svelte` with the wired-up version**

```svelte
<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import Sidebar from '$components/Sidebar.svelte';
    import { KeyHintBar } from '$components/ui/index';
    import { theme } from '$lib/theme.svelte';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';

    let { children } = $props();

    onMount(() => {
        theme.init();
        uiPrefs.init();
        const unmount = keyboard.mount();
        const cleanup = keyboard.smartRegister([
            ['meta+digit1', () => { goto('/'); return true; }],
            ['meta+digit2', () => { goto('/convert'); return true; }],
            ['meta+digit3', () => { goto('/settings'); return true; }],
            ['meta+digit4', () => { goto('/about'); return true; }],
            ['meta+keyb', () => { sidebar.toggle(); return true; }],
        ]);
        return () => { unmount(); cleanup(); };
    });
</script>

<div
    class="flex h-screen flex-col overflow-hidden bg-thasia-bg text-thasia-text"
    use:mountedHint={[
        ['meta+digit1', 'Home'],
        ['meta+digit2', 'Convert'],
        ['meta+digit3', 'Settings'],
        ['meta+digit4', 'About'],
        ['meta+keyb', 'Sidebar'],
    ]}
>
    <!-- macOS title bar -->
    <div
        class="titlebar h-8 flex-shrink-0 border-b border-thasia-border bg-thasia-surface"
        data-tauri-drag-region
    ></div>

    <div class="flex flex-1 overflow-hidden">
        <Sidebar />
        <main class="flex flex-1 flex-col overflow-hidden">
            <div class="flex-1 overflow-auto">
                {@render children()}
            </div>
            <KeyHintBar />
        </main>
    </div>
</div>
```

- [ ] **Step 2: Start the dev server and verify**

```bash
cd /Users/Luca/Documents/Projekte/Thasia && npm run tauri dev
```

Check:
- Hint bar appears at the bottom showing `⌘1 Home  ⌘2 Convert  ⌘3 Settings  ⌘4 About  ⌘B Sidebar`
- `⌘1` navigates to Home, `⌘2` to Convert, `⌘3` to Settings, `⌘4` to About
- `⌘B` toggles the sidebar

- [ ] **Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat: wire global keyboard shortcuts and hint bar into layout"
```

---

## Task 8: Wizard navigation shortcuts

**Files:**
- Modify: `src/routes/convert/+page.svelte`

- [ ] **Step 1: Replace `src/routes/convert/+page.svelte`**

```svelte
<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { STEPS, activeSteps } from '$lib/wizard/steps';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

    onMount(() => {
        sidebar.enterWizard();
        if (!wizard.currentStepId) wizard.currentStepId = 'source';

        const raw = localStorage.getItem('thasia:settings');
        if (raw && !wizard.sourcePath) {
            try {
                const d = JSON.parse(raw);
                if (d.imageFormat) wizard.imageFormat = d.imageFormat;
                if (d.container) wizard.container = d.container;
                if (d.direction) wizard.direction = d.direction;
                if (d.bundle) wizard.bundle = d.bundle;
                if (d.volumeSeparator !== undefined) wizard.volumeSeparator = d.volumeSeparator;
                if (d.hideSingleVolume !== undefined) wizard.hideSingleVolume = d.hideSingleVolume;
                if (d.createDirectory !== undefined) wizard.createDirectory = d.createDirectory;
                if (d.maxWidth !== undefined) wizard.maxWidth = d.maxWidth;
            } catch {}
        }

        document.addEventListener('wizard:goto', handleGoto);

        const cleanup = keyboard.smartRegister([
            ['alt+arrowright', (e) => { e.preventDefault(); goNext(); return true; }],
            ['alt+arrowleft', (e) => { e.preventDefault(); goBack(); return true; }],
        ]);

        return () => {
            cleanup();
            document.removeEventListener('wizard:goto', handleGoto);
        };
    });

    onDestroy(() => {
        sidebar.exitWizard();
    });

    function handleGoto(e: Event) {
        const id = (e as CustomEvent<string>).detail;
        if (wizard.completedStepIds.has(id)) wizard.currentStepId = id;
    }

    let active = $derived(activeSteps(wizard));
    let currentStep = $derived(active.find((s) => s.id === wizard.currentStepId) ?? active[0]);
    let currentIndex = $derived(active.findIndex((s) => s.id === wizard.currentStepId));

    function goNext() {
        if (!currentStep) return;
        wizard.markComplete(currentStep.id);
        const nextIndex = currentIndex + 1;
        if (nextIndex < active.length) wizard.currentStepId = active[nextIndex].id;
        if (active[nextIndex]?.id === 'convert') sidebar.collapseForConversion();
    }

    function goBack() {
        if (currentIndex > 0) wizard.currentStepId = active[currentIndex - 1].id;
    }
</script>

<div
    class="flex h-full flex-col"
    use:mountedHint={[
        ['alt+arrowright', 'Next step'],
        ['alt+arrowleft', 'Back'],
    ]}
>
    {#if currentStep}
        {#key currentStep.id}
            <currentStep.component onNext={goNext} onBack={goBack} />
        {/key}
    {/if}
</div>
```

- [ ] **Step 2: Verify in dev server**

Open the wizard, press `Alt+→` to advance steps and `Alt+←` to go back. The hint bar should show `⌥→ Next step  ⌥← Back` while in the wizard (in addition to the global hints).

- [ ] **Step 3: Commit**

```bash
git add src/routes/convert/+page.svelte
git commit -m "feat: add Alt+arrow wizard navigation shortcuts"
```

---

## Task 9: Settings — showKeyHints toggle

**Files:**
- Modify: `src/routes/settings/+page.svelte`

- [ ] **Step 1: Replace `src/routes/settings/+page.svelte`**

The changes are: add `showKeyHints` to `Defaults`, load it in `onMount`, save it in `save()`, and add the Interface panel below the existing two-column grid.

```svelte
<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { onMount } from 'svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import {
        IconCheck,
        IconPhoto,
        IconFileZip,
        IconDirection,
        IconStack,
        IconFolderPlus,
        IconRuler,
        IconKeyboard,
    } from '@tabler/icons-svelte';
    import { Button, Toggle, Input, SegmentedControl } from '$components/ui/index';
    import { duration } from '$lib/transitions';

    const KEY = 'thasia:settings';

    type Defaults = {
        imageFormat: 'avif' | 'webp' | 'original';
        container: 'cbz' | 'epub' | 'raw';
        direction: 'ltr' | 'rtl';
        bundle: 'auto' | 'flatten';
        volumeSeparator: string;
        hideSingleVolume: boolean;
        createDirectory: boolean;
        maxWidth: number | null;
        showKeyHints: boolean;
    };

    let defaults = $state<Defaults>({
        imageFormat: 'avif',
        container: 'cbz',
        direction: 'ltr',
        bundle: 'auto',
        volumeSeparator: ' - ',
        hideSingleVolume: false,
        createDirectory: false,
        maxWidth: null,
        showKeyHints: true,
    });

    let saved = $state(false);
    let maxWidthEnabled = $state(false);

    onMount(() => {
        const raw = localStorage.getItem(KEY);
        if (raw) {
            try {
                const parsed = JSON.parse(raw);
                Object.assign(defaults, parsed);
                maxWidthEnabled = defaults.maxWidth !== null;
            } catch {}
        }
    });

    function save() {
        if (!maxWidthEnabled) defaults.maxWidth = null;
        localStorage.setItem(KEY, JSON.stringify(defaults));
        wizard.imageFormat = defaults.imageFormat;
        wizard.container = defaults.container;
        wizard.direction = defaults.direction;
        wizard.bundle = defaults.bundle;
        wizard.volumeSeparator = defaults.volumeSeparator;
        wizard.hideSingleVolume = defaults.hideSingleVolume;
        wizard.createDirectory = defaults.createDirectory;
        wizard.maxWidth = defaults.maxWidth;
        uiPrefs.showKeyHints = defaults.showKeyHints;
        saved = true;
        setTimeout(() => (saved = false), 2000);
    }

    const collapse = { duration: duration.base, easing: cubicInOut };

    const formatHint: Record<string, string> = {
        avif: 'Best compression, slower — ideal for archiving',
        webp: 'Good compression, widely supported',
        original: 'No re-encoding — fastest, preserves originals',
    };
</script>

<div class="flex h-full flex-col overflow-hidden">
    <div class="mx-auto flex min-h-0 w-full max-w-5xl flex-1 flex-col gap-6 px-8 py-8">
        <!-- Header -->
        <div class="flex flex-shrink-0 items-center justify-between">
            <div>
                <h1 class="text-xl font-bold">Settings</h1>
                <p class="mt-0.5 text-sm text-thasia-muted">
                    Default values pre-filled in each new conversion
                </p>
            </div>
            <Button variant="primary" size="lg" onclick={save}>
                {#if saved}<IconCheck size={16} />{/if}
                {saved ? 'Saved' : 'Save defaults'}
            </Button>
        </div>

        <!-- Encoding + Output panels -->
        <div class="grid min-h-0 flex-1 grid-cols-2 gap-4">
            <!-- LEFT: Encoding -->
            <div class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                <div class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5">
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase">Encoding</span>
                </div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconPhoto size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Image Format</span>
                    </div>
                    <SegmentedControl
                        options={[
                            { value: 'avif', label: 'AVIF' },
                            { value: 'webp', label: 'WebP' },
                            { value: 'original', label: 'Original' },
                        ]}
                        bind:value={defaults.imageFormat}
                    />
                    <p class="text-xs text-thasia-muted">{formatHint[defaults.imageFormat]}</p>
                </div>

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2">
                            <IconRuler size={14} class="flex-shrink-0 text-thasia-muted" />
                            <div>
                                <div class="text-sm font-medium">Max Width</div>
                                <div class="text-xs text-thasia-muted">Downscale wider images (px)</div>
                            </div>
                        </div>
                        <Toggle
                            bind:checked={maxWidthEnabled}
                            onchange={(v) => {
                                if (!v) defaults.maxWidth = null;
                                else defaults.maxWidth = 1920;
                            }}
                        />
                    </div>
                    {#if maxWidthEnabled}
                        <div transition:slide={collapse}>
                            <Input
                                type="number"
                                min="100"
                                max="9999"
                                bind:value={defaults.maxWidth as number}
                                hint="Common values: 1200, 1440, 1920"
                            />
                        </div>
                    {/if}
                </div>
            </div>

            <!-- RIGHT: Output -->
            <div class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                <div class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5">
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase">Output</span>
                </div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconFileZip size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Container</span>
                    </div>
                    <SegmentedControl
                        options={[
                            { value: 'cbz', label: 'CBZ' },
                            { value: 'epub', label: 'EPUB' },
                            { value: 'raw', label: 'Raw' },
                        ]}
                        bind:value={defaults.container}
                    />
                    {#if defaults.container === 'epub'}
                        <div class="flex items-center justify-between" transition:slide={collapse}>
                            <div class="flex items-center gap-1.5">
                                <IconDirection size={13} class="flex-shrink-0 text-thasia-muted" />
                                <span class="text-xs text-thasia-muted">Reading direction</span>
                            </div>
                            <SegmentedControl
                                options={[
                                    { value: 'ltr', label: 'LTR' },
                                    { value: 'rtl', label: 'RTL' },
                                ]}
                                bind:value={defaults.direction}
                            />
                        </div>
                    {/if}
                </div>

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconStack size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Bundling</span>
                    </div>
                    <SegmentedControl
                        options={[
                            { value: 'auto', label: 'Auto' },
                            { value: 'flatten', label: 'Flatten' },
                        ]}
                        bind:value={defaults.bundle}
                    />
                    {#if defaults.bundle === 'auto'}
                        <div class="flex flex-col gap-3" transition:slide={collapse}>
                            <Input
                                label="Volume separator"
                                bind:value={defaults.volumeSeparator}
                                hint={`e.g. "Manga${defaults.volumeSeparator}1"`}
                            />
                            <Toggle
                                bind:checked={defaults.hideSingleVolume}
                                label="Omit volume number when only one volume"
                            />
                        </div>
                    {/if}
                </div>

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex items-center justify-between gap-4 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconFolderPlus size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Create subdirectory</span>
                    </div>
                    <Toggle bind:checked={defaults.createDirectory} />
                </div>
            </div>
        </div>

        <!-- Interface panel — full width below the grid -->
        <div class="flex-shrink-0 overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <div class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase">Interface</span>
            </div>
            <div class="flex items-center justify-between px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconKeyboard size={14} class="flex-shrink-0 text-thasia-muted" />
                    <div>
                        <div class="text-sm font-medium">Keyboard hint bar</div>
                        <div class="text-xs text-thasia-muted">Show shortcut hints at the bottom of the window</div>
                    </div>
                </div>
                <Toggle bind:checked={defaults.showKeyHints} />
            </div>
        </div>
    </div>
</div>
```

- [ ] **Step 2: Verify in dev server**

Open Settings. Scroll to bottom — Interface panel should appear with the keyboard toggle. Toggle it off, click Save — hint bar disappears. Toggle on, Save — it returns.

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat: add showKeyHints toggle to settings Interface panel"
```

---

## Task 10: Step 3 — Image format shortcuts

**Files:**
- Modify: `src/components/wizard/Step3ImageFormat.svelte`

- [ ] **Step 1: Add keyboard imports and mountedHint to `Step3ImageFormat.svelte`**

Add to the `<script>` block (after existing imports):

```ts
import { onMount, onDestroy } from 'svelte';
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';
```

Add in `<script>`, before closing `</script>`:

```ts
let cleanupKb: (() => void) | undefined;
onMount(() => {
    cleanupKb = keyboard.smartRegister([
        ['keya', () => { wizard.imageFormat = 'avif'; return true; }],
        ['keyw', () => { wizard.imageFormat = 'webp'; return true; }],
        ['keyo', () => { wizard.imageFormat = 'original'; return true; }],
    ]);
});
onDestroy(() => cleanupKb?.());
```

Change the root `<div>` opening tag from:

```svelte
<div class="flex h-full flex-col">
```

to:

```svelte
<div class="flex h-full flex-col" use:mountedHint={[['keya', 'AVIF'], ['keyw', 'WebP'], ['keyo', 'Original']]}>
```

- [ ] **Step 2: Verify in dev server**

Navigate to Step 3 (Image Format). Press `A` — AVIF selects. Press `W` — WebP selects. Press `O` — Original selects. Hint bar shows the three hints.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step3ImageFormat.svelte
git commit -m "feat: add A/W/O keyboard shortcuts to image format step"
```

---

## Task 11: Step 4 — Container shortcuts

**Files:**
- Modify: `src/components/wizard/Step4Container.svelte`

- [ ] **Step 1: Add keyboard imports and bindings to `Step4Container.svelte`**

Add to `<script>` block:

```ts
import { onMount, onDestroy } from 'svelte';
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';

let cleanupKb: (() => void) | undefined;
onMount(() => {
    cleanupKb = keyboard.smartRegister([
        ['keyc', () => { wizard.container = 'cbz'; return true; }],
        ['keye', () => { wizard.container = 'epub'; return true; }],
        ['keyr', () => { wizard.container = 'raw'; return true; }],
    ]);
});
onDestroy(() => cleanupKb?.());
```

Change root `<div>` opening tag to:

```svelte
<div class="flex h-full flex-col" use:mountedHint={[['keyc', 'CBZ'], ['keye', 'EPUB'], ['keyr', 'Raw']]}>
```

- [ ] **Step 2: Verify in dev server**

Navigate to Step 4. Press `C` → CBZ, `E` → EPUB, `R` → Raw. Hint bar updates.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step4Container.svelte
git commit -m "feat: add C/E/R keyboard shortcuts to container step"
```

---

## Task 12: Step 5 — Direction shortcuts

**Files:**
- Modify: `src/components/wizard/Step5Direction.svelte`

- [ ] **Step 1: Add keyboard imports and bindings to `Step5Direction.svelte`**

Add to `<script>` block:

```ts
import { onMount, onDestroy } from 'svelte';
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';

let cleanupKb: (() => void) | undefined;
onMount(() => {
    cleanupKb = keyboard.smartRegister([
        ['keyl', () => { wizard.direction = 'ltr'; return true; }],
        ['keyr', () => { wizard.direction = 'rtl'; return true; }],
    ]);
});
onDestroy(() => cleanupKb?.());
```

Change root `<div>` opening tag to:

```svelte
<div class="flex h-full flex-col" use:mountedHint={[['keyl', 'LTR'], ['keyr', 'RTL']]}>
```

- [ ] **Step 2: Verify in dev server**

Navigate to Step 5 (only visible when EPUB is selected). Press `L` → LTR, `R` → RTL.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step5Direction.svelte
git commit -m "feat: add L/R keyboard shortcuts to direction step"
```

---

## Task 13: Step 6 — Bundling shortcuts

**Files:**
- Modify: `src/components/wizard/Step6Bundling.svelte`

- [ ] **Step 1: Add keyboard imports and bindings to `Step6Bundling.svelte`**

Add to `<script>` block:

```ts
import { onMount, onDestroy } from 'svelte';
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';

let cleanupKb: (() => void) | undefined;
onMount(() => {
    cleanupKb = keyboard.smartRegister([
        ['keya', () => { wizard.bundle = 'auto'; return true; }],
        ['keyf', () => { wizard.bundle = 'flatten'; return true; }],
    ]);
});
onDestroy(() => cleanupKb?.());
```

Change root `<div>` opening tag to:

```svelte
<div class="flex h-full flex-col" use:mountedHint={[['keya', 'Auto'], ['keyf', 'Flatten']]}>
```

- [ ] **Step 2: Verify in dev server**

Navigate to Step 6. Press `A` → Auto, `F` → Flatten.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step6Bundling.svelte
git commit -m "feat: add A/F keyboard shortcuts to bundling step"
```

---

## Task 14: Step 7 — Page editor volume navigation

**Files:**
- Modify: `src/components/wizard/Step7PageEditor.svelte`

The Page Editor has multiple volumes in a left panel. `←`/`→` navigate between volumes (previous/next). There is no single "selected page" concept in the current UI model, so per-page keyboard navigation is out of scope.

- [ ] **Step 1: Add keyboard imports and bindings to `Step7PageEditor.svelte`**

Add to `<script>` block (after existing imports):

```ts
import { onMount, onDestroy } from 'svelte';
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';

let cleanupKb: (() => void) | undefined;
onMount(() => {
    cleanupKb = keyboard.smartRegister([
        ['arrowleft', (e) => {
            e.preventDefault();
            if (activeVolumeIndex > 0) activeVolumeIndex--;
            return true;
        }],
        ['arrowright', (e) => {
            e.preventDefault();
            if (activeVolumeIndex < volumes.length - 1) activeVolumeIndex++;
            return true;
        }],
    ]);
});
onDestroy(() => cleanupKb?.());
```

Change root `<div>` opening tag from:

```svelte
<div class="flex h-full gap-0">
```

to:

```svelte
<div class="flex h-full gap-0" use:mountedHint={[['arrowleft', 'Prev volume'], ['arrowright', 'Next volume']]}>
```

- [ ] **Step 2: Verify in dev server**

Navigate to the Page Editor with multiple volumes. Press `←` / `→` — the active volume changes. Hint bar shows the two hints.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step7PageEditor.svelte
git commit -m "feat: add arrow key volume navigation to page editor"
```

---

## Task 15: Step 9 — Convert Enter to start over

**Files:**
- Modify: `src/components/wizard/Step9Convert.svelte`

- [ ] **Step 1: Add keyboard import and Enter binding to `Step9Convert.svelte`**

Add to `<script>` block after existing imports:

```ts
import { keyboard } from '$lib/keyboard';
import { mountedHint } from '$lib/keyhint.svelte';
```

The step already has `onMount`/`onDestroy` for event listeners. Extend the existing `onMount` to also register the keyboard shortcut. Find the existing `unlisteners` array pattern and add the keyboard cleanup alongside it.

Replace the existing `onMount` return (which doesn't exist — `onMount` currently has no cleanup return) and change to:

```ts
onMount(async () => {
    // ... existing event listener registrations (unlisteners.push(...)) ...

    const cleanupKb = keyboard.smartRegister([
        ['enter', () => {
            if (status === 'done') { wizard.reset(); goto('/'); }
            return true;
        }],
    ]);

    // ... existing status = 'converting' and commands.convert call ...

    return () => {
        unlisteners.forEach((u) => u());
        cleanupKb();
    };
});
```

The `onDestroy` that calls `unlisteners.forEach` should be removed since cleanup is now in the `onMount` return. Keep `onDestroy` only if it was doing something else; in the current code it is:

```ts
onDestroy(() => {
    unlisteners.forEach((u) => u());
});
```

Replace it entirely — the `onMount` return handles cleanup now, no `onDestroy` needed.

Change the root `<div>` opening tag from:

```svelte
<div class="flex h-full flex-col">
```

to:

```svelte
<div class="flex h-full flex-col" use:mountedHint={status === 'done' ? [['enter', 'Start over']] : []}>
```

- [ ] **Step 2: Verify in dev server**

Complete a conversion. When it finishes, hint bar shows `↩ Start over`. Press Enter — wizard resets and navigates to Home.

- [ ] **Step 3: Commit**

```bash
git add src/components/wizard/Step9Convert.svelte
git commit -m "feat: add Enter shortcut to start over after conversion"
```

---

## Self-Review

**Spec coverage:**
- ✅ `keyboard.ts` — KeyboardManager with `register`, `unregister`, `smartRegister`, `mount`
- ✅ `keyhint.svelte.ts` — KeyHintState + `mountedHint` action + `handleKeyHint` action
- ✅ `ui-prefs.svelte.ts` — `showKeyHints` reactive singleton
- ✅ `KeyComboDisplay.svelte` — OS-aware combo rendering using `Kbd`
- ✅ `KeyHintBar.svelte` — animated strip, reads `keyHint.get()`, respects `showKeyHints`
- ✅ Global shortcuts: `⌘1`–`⌘4`, `⌘B`
- ✅ Wizard nav: `Alt+→`, `Alt+←`
- ✅ Step 3: A/W/O
- ✅ Step 4: C/E/R
- ✅ Step 5: L/R
- ✅ Step 6: A/F
- ✅ Step 7: ←/→ volume navigation
- ✅ Step 9: Enter to start over
- ✅ Settings Interface panel with showKeyHints toggle

**Type consistency:** `mountedHint` takes `[string, string][]` everywhere. `keyboard.smartRegister` takes `[string, callback, id?][]` everywhere. ✅

**Placeholder scan:** All steps contain complete code. ✅
