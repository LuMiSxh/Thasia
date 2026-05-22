# Anasthasia — Component Reference & Design Guide

> Complete API reference for all components, stores, transitions, and design guidelines.

---

## Table of Contents

- [Design Guidelines](#design-guidelines)
- [Setup](#setup)
- [Theming & Flavours](#theming--flavours)
- [Components](#components)
  - [Alert](#alert)
  - [Badge](#badge)
  - [Button](#button)
  - [Card](#card)
  - [Dialog](#dialog)
  - [FieldRow](#fieldrow)
  - [Input](#input)
  - [Kbd](#kbd)
  - [KeyComboDisplay](#keycombodisplay)
  - [KeyHintBar](#keyhintbar)
  - [Panel](#panel)
  - [PathDisplay](#pathdisplay)
  - [ProgressBar](#progressbar)
  - [SectionLabel](#sectionlabel)
  - [SegmentedControl](#segmentedcontrol)
  - [Select](#select)
  - [ToastProvider](#toastprovider)
  - [Toggle](#toggle)
- [Stores](#stores)
- [Transitions](#transitions)
- [Utility Classes](#utility-classes)
- [Writing a Custom Flavour](#writing-a-custom-flavour)

---

## Design Guidelines

### Philosophy

Anasthasia is a **desktop-first** component library. It prioritises density, keyboard navigation, and a precise visual language over responsiveness or mobile ergonomics. Components are designed to compose in tight layouts — settings panels, wizard steps, toolbars, data-heavy screens.

### Density

Keep layouts dense. Default `gap` between form fields is `gap-4` (1rem). Between grouped items of the same type, use `gap-2` or `gap-3`. Do not add extra padding around components — they already include their own internal spacing.

### Hierarchy

Use the four surface tokens in order of depth:

| Token       | Tailwind class             | Use for                                  |
| ----------- | -------------------------- | ---------------------------------------- |
| `--bg`      | `bg-anasthasia-bg`         | Page / window background                 |
| `--surface` | `bg-anasthasia-surface`    | Primary content containers               |
| `--panel`   | `bg-anasthasia-panel`      | Panel headers, sidebars, nested sections |
| `--border`  | `border-anasthasia-border` | All borders and dividers                 |

Never skip a level. A card on a panel on a surface on a background — not a card directly on background.

### Typography

All semantic HTML elements (`h1`–`h6`, `p`, `small`, `code`, `a`, `hr`) are styled globally by Anasthasia's base layer. Do not apply manual text classes to these elements. Use them semantically and let the tokens do the work.

For non-semantic text roles, use the utility classes:

| Class                 | Use for                               |
| --------------------- | ------------------------------------- |
| `.anasthasia-label`   | Uppercase section labels, form labels |
| `.anasthasia-caption` | Hints, helper text, timestamps        |
| `.anasthasia-body`    | Body copy inside components           |
| `.anasthasia-mono`    | Paths, codes, technical values        |

### Colour

Never use hardcoded colours in application code. Always reference design tokens:

- **Accent** (`--accent`, `text-anasthasia-accent`) — interactive elements, highlights, active states
- **Accent strong** (`--accent-strong`) — pressed states, darker highlights
- **Muted** (`--muted`, `text-anasthasia-muted`) — secondary text, placeholders, disabled labels
- **Red** (`text-red-400`, `bg-red-600/10`) — destructive actions and errors only

For gradients and elevated effects, use the `.text-accent-gradient` and `.bevel-accent` utilities provided by the base styles.

### Actions and Intent

Map button variants to intent consistently:

| Variant     | Intent                                       |
| ----------- | -------------------------------------------- |
| `primary`   | The single most important action on a screen |
| `secondary` | Default / neutral actions                    |
| `ghost`     | Tertiary actions, cancel, dismiss            |
| `danger`    | Destructive and irreversible actions only    |

Only one `primary` button per view. Never two `primary` buttons side by side.

### Feedback

- Use `Alert` for persistent, in-context feedback tied to a specific section.
- Use `toast` for transient feedback from async operations (saves, completions, errors).
- Use `Badge` for static state labels, not for actions.
- Use `ProgressBar` only for measurable progress — not as a decorative element.

### Motion

Prefer the named transitions from Anasthasia over custom animations. All named transitions are tuned to 150–300ms with `cubicOut` easing to feel native and fast.

- `riseIn` / `riseOut` — panels, dialogs, popovers
- `softCollapse` — conditional form regions
- `glassCollapse` — inline chips and status tags
- `sendPill` / `receivePill` — active indicator movement between tabs/segments

---

## Setup

### 1. Install

```sh
pnpm add github:LuMiSxh/Anasthasia#v0.1.0
```

### 2. Import fonts (root layout)

```svelte
<script lang="ts">
	import 'anasthasia/bootstrap';
	import './app.css';
	const { children } = $props();
</script>

{@render children()}
```

### 3. Import styles and a flavour (root stylesheet)

```css
@import 'tailwindcss';
@import 'anasthasia/styles';
@import 'anasthasia/flavours/imperial';
```

`anasthasia/styles` ships **no colours** — a flavour import is required. Without one, the base neutral grey fallback is used.

### 4. Use components

```svelte
<script lang="ts">
	import { Button, Card, Input } from 'anasthasia';
</script>
```

---

## Theming & Flavours

### Tokens

All visual values are CSS custom properties. Anasthasia exposes eight semantic colour tokens and one gradient token:

| Variable            | Tailwind utility                | Purpose                                                    |
| ------------------- | ------------------------------- | ---------------------------------------------------------- |
| `--bg`              | `bg-anasthasia-bg`              | Page background                                            |
| `--surface`         | `bg-anasthasia-surface`         | Card / container background                                |
| `--panel`           | `bg-anasthasia-panel`           | Panel, header, sidebar                                     |
| `--border`          | `border-anasthasia-border`      | Borders and dividers                                       |
| `--text`            | `text-anasthasia-text`          | Primary text                                               |
| `--muted`           | `text-anasthasia-muted`         | Secondary / placeholder text                               |
| `--accent`          | `text-anasthasia-accent`        | Interactive / highlight colour                             |
| `--accent-strong`   | `text-anasthasia-accent-strong` | Pressed / stronger accent                                  |
| `--accent-gradient` | `bg-accent-gradient`            | Gradient (used in primary button, `.text-accent-gradient`) |

### Light / Dark Mode

Dark mode is toggled by adding the `.dark` class to `<html>`. Use the `theme` store:

```svelte
<script lang="ts">
	import { theme } from 'anasthasia';
</script>

<button onclick={() => theme.toggle()}>
	{theme.dark ? 'Light mode' : 'Dark mode'}
</button>
```

`theme.init()` must be called once on mount to restore the persisted preference:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { theme } from 'anasthasia';
	onMount(() => theme.init());
</script>
```

### Flavours

A flavour is a CSS file that defines all tokens for both `:root` (light) and `.dark`. Import exactly one flavour per app.

**Available flavours:**

| Import                         | Description                                                               |
| ------------------------------ | ------------------------------------------------------------------------- |
| `anasthasia/flavours/imperial` | Luxury Cathedral light + Immortal Abyssal dark — gold, generous rounding  |
| `anasthasia/flavours/crimson`  | Crimson Dawn light + Crimson Noir dark — deep red, sharp precise rounding |

**Creating a custom flavour:**

```css
/* my-flavour.css */
:root {
	--bg: #f8f8f8;
	--surface: #ffffff;
	--panel: #f2f2f2;
	--border: rgba(0, 0, 0, 0.1);
	--text: #111111;
	--muted: #666666;
	--accent: #3b82f6;
	--accent-strong: #1d4ed8;
	--accent-gradient: linear-gradient(135deg, #93c5fd 0%, #3b82f6 50%, #1d4ed8 100%);
}

.dark {
	/* ... dark variants */
}
```

---

## Components

---

### Alert

Inline feedback strip tied to a specific section of the UI. For transient feedback, use `toast` instead.

```svelte
<Alert variant="warning" title="Unsaved changes">Your settings have not been saved yet.</Alert>
```

| Prop       | Type                                           | Default  | Description                  |
| ---------- | ---------------------------------------------- | -------- | ---------------------------- |
| `variant`  | `'info' \| 'success' \| 'warning' \| 'danger'` | `'info'` | Colour variant               |
| `title`    | `string`                                       | —        | Bold title above the message |
| `class`    | `string`                                       | —        | Extra CSS classes            |
| `children` | `Snippet`                                      | required | Alert body content           |

---

### Badge

Static label for status, version, or category. Not interactive.

```svelte
<Badge variant="accent">v1.0.0</Badge>
<Badge variant="success">Ready</Badge>
<Badge variant="danger">Failed</Badge>
```

| Prop       | Type                                                                    | Default     | Description       |
| ---------- | ----------------------------------------------------------------------- | ----------- | ----------------- |
| `variant`  | `'default' \| 'accent' \| 'success' \| 'warning' \| 'danger' \| 'mono'` | `'default'` | Colour variant    |
| `class`    | `string`                                                                | —           | Extra CSS classes |
| `children` | `Snippet`                                                               | required    | Badge text        |

---

### Button

The primary action element. Extends all native `HTMLButtonAttributes`.

```svelte
<Button variant="primary" onclick={save}>Save</Button>
<Button variant="danger" size="sm">Delete</Button>
<Button loading={isSaving} loadingLabel="Saving…">Save</Button>
<Button success={saved} successLabel="Saved!">Save</Button>
```

| Prop           | Type                                              | Default       | Description                                                  |
| -------------- | ------------------------------------------------- | ------------- | ------------------------------------------------------------ |
| `variant`      | `'primary' \| 'secondary' \| 'ghost' \| 'danger'` | `'secondary'` | Visual intent                                                |
| `size`         | `'sm' \| 'md' \| 'lg'`                            | `'md'`        | Button size                                                  |
| `loading`      | `boolean`                                         | `false`       | Shows a pulsing dot and `loadingLabel`, disables interaction |
| `success`      | `boolean`                                         | `false`       | Shows `successLabel`, disables interaction                   |
| `loadingLabel` | `string`                                          | `'Working'`   | Text shown in loading state                                  |
| `successLabel` | `string`                                          | `'Done'`      | Text shown in success state                                  |
| `disabled`     | `boolean`                                         | `false`       | Disables the button                                          |
| `class`        | `string`                                          | —             | Extra CSS classes                                            |
| `children`     | `Snippet`                                         | required      | Button label                                                 |

**Design note:** Use at most one `primary` button per view. `loading` and `success` are mutually exclusive with `disabled` — all three prevent interaction.

---

### Card

A surface container. Renders as a `<div>` by default; renders as a `<button>` when `onclick` is provided.

```svelte
<!-- Static -->
<Card>
	<h3>Title</h3>
	<p>Description</p>
</Card>

<!-- Interactive -->
<Card onclick={() => select(item)}>
	<div class="flex justify-between">
		<span>{item.name}</span>
		<Badge>{item.status}</Badge>
	</div>
</Card>
```

| Prop       | Type         | Default  | Description                                 |
| ---------- | ------------ | -------- | ------------------------------------------- |
| `onclick`  | `() => void` | —        | Makes card clickable; renders as `<button>` |
| `class`    | `string`     | —        | Extra CSS classes                           |
| `children` | `Snippet`    | required | Card content                                |

---

### Dialog

A modal dialog with backdrop, focus trap, and keyboard handling. Control visibility with `bind:open`.

```svelte
<script lang="ts">
	let open = $state(false);
</script>

<Button onclick={() => (open = true)}>Open</Button>

<Dialog bind:open title="Confirm" description="This cannot be undone.">
	<p>Are you sure you want to delete this item?</p>

	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
		<Button variant="danger" onclick={confirm}>Delete</Button>
	{/snippet}
</Dialog>
```

| Prop              | Type         | Default  | Description                       |
| ----------------- | ------------ | -------- | --------------------------------- |
| `open`            | `boolean`    | required | Bindable — controls visibility    |
| `title`           | `string`     | —        | Dialog heading                    |
| `description`     | `string`     | —        | Subtitle under heading            |
| `closeOnBackdrop` | `boolean`    | `true`   | Close when clicking the backdrop  |
| `onclose`         | `() => void` | —        | Called whenever the dialog closes |
| `class`           | `string`     | —        | Extra CSS classes on the panel    |
| `children`        | `Snippet`    | required | Main dialog body                  |
| `footer`          | `Snippet`    | —        | Footer area (action buttons)      |
| `closeIcon`       | `Snippet`    | —        | Replaces the default ✕ icon       |

**Keyboard:** `Escape` closes. `Tab` / `Shift+Tab` cycles focus within the dialog.

---

### FieldRow

A horizontal layout pairing a label+hint on the left with a control on the right. Use inside forms to align controls in a consistent two-column layout.

```svelte
<FieldRow label="Output format" hint="Affects file size and compatibility.">
	<SegmentedControl options={formatOptions} bind:value={format} />
</FieldRow>
```

| Prop       | Type      | Default  | Description                    |
| ---------- | --------- | -------- | ------------------------------ |
| `label`    | `string`  | required | Left-side label                |
| `hint`     | `string`  | —        | Secondary text below label     |
| `class`    | `string`  | —        | Extra CSS classes              |
| `children` | `Snippet` | required | Right-side control             |
| `meta`     | `Snippet` | —        | Additional metadata after hint |

---

### Input

A text input with label, hint, and error state. Extends all native `HTMLInputAttributes`.

```svelte
<Input label="Project name" bind:value={name} hint="Used as the output filename." />
<Input label="Path" value={badPath} error="Path contains invalid characters." />
<Input label="Search" placeholder="Type to filter…" type="search" />
```

| Prop    | Type     | Default | Description                              |
| ------- | -------- | ------- | ---------------------------------------- |
| `label` | `string` | —       | Input label                              |
| `hint`  | `string` | —       | Helper text (hidden when `error` is set) |
| `error` | `string` | —       | Error message; applies red border        |
| `value` | `string` | —       | Bindable input value                     |
| `class` | `string` | —       | Extra CSS classes on the wrapper         |

All native `HTMLInputAttributes` (`type`, `placeholder`, `disabled`, `required`, `oninput`, etc.) are forwarded.

---

### Kbd

Renders a single keyboard key in styled monospace.

```svelte
<Kbd>Esc</Kbd>
<Kbd>⌘</Kbd>
<Kbd>Enter</Kbd>
```

| Prop       | Type      | Default  | Description |
| ---------- | --------- | -------- | ----------- |
| `children` | `Snippet` | required | Key label   |

---

### KeyComboDisplay

Renders a full keyboard shortcut (modifier + key) with platform-aware symbols. Parses a combo string into individual `Kbd` tokens.

```svelte
<KeyComboDisplay combo="meta+keys" />
<KeyComboDisplay combo="ctrl+shift+arrowdown" />
<KeyComboDisplay combo="shift+enter" />
```

| Prop    | Type     | Default  | Description                        |
| ------- | -------- | -------- | ---------------------------------- |
| `combo` | `string` | required | Shortcut string (see format below) |

**Combo format:** `modifier+modifier+key`, all lowercase, joined by `+`.

Supported modifiers: `meta`, `ctrl`, `alt`, `shift`
Supported keys: `enter`, `escape`, `backspace`, `tab`, `space`, `arrowup`, `arrowdown`, `arrowleft`, `arrowright`
Alpha keys: `keya`–`keyz` (e.g. `keys` → S)
Digit keys: `digit0`–`digit9`

---

### KeyHintBar

A fixed bottom bar that displays registered keyboard shortcuts. Reads from the `keyHint` store. Place once in the root layout.

```svelte
<!-- In your layout or page root -->
<KeyHintBar />
```

No props. Visibility is controlled by `uiPrefs.showKeyHints`. Register hints with `keyHint.register()`.

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { keyHint, uiPrefs } from 'anasthasia';

	onMount(() => {
		uiPrefs.showKeyHints = true;
		return keyHint.register([
			['meta+keys', 'Save'],
			['meta+keyz', 'Undo'],
			['escape', 'Cancel']
		]);
	});
</script>
```

`keyHint.register()` returns an unregister function — call it from the `onMount` return to clean up when the component is destroyed.

---

### Panel

A bordered content section with an optional header (label + title + actions). The primary layout container for grouped UI.

```svelte
<Panel label="Export" title="Output Settings">
	{#snippet actions()}
		<Badge variant="mono">CBZ</Badge>
	{/snippet}

	<div class="flex flex-col gap-4">
		<!-- content -->
	</div>
</Panel>
```

| Prop       | Type      | Default  | Description                                  |
| ---------- | --------- | -------- | -------------------------------------------- |
| `label`    | `string`  | —        | Small uppercase eyebrow label                |
| `title`    | `string`  | —        | Main heading                                 |
| `class`    | `string`  | —        | Extra CSS classes                            |
| `children` | `Snippet` | required | Panel body                                   |
| `actions`  | `Snippet` | —        | Right-side header controls (badges, buttons) |

---

### PathDisplay

Displays a filesystem path or technical string in monospace with an empty state.

```svelte
<PathDisplay value="/Users/Luca/Documents/output.cbz" />
<PathDisplay empty="No destination selected" />
```

| Prop    | Type             | Default               | Description                  |
| ------- | ---------------- | --------------------- | ---------------------------- |
| `value` | `string \| null` | —                     | The path or value to show    |
| `empty` | `string`         | `'No value selected'` | Fallback when value is empty |
| `class` | `string`         | —                     | Extra CSS classes            |

---

### ProgressBar

A horizontal progress indicator. Value is clamped between 0 and 1.

```svelte
<ProgressBar value={0.65} variant="accent" />
<ProgressBar value={progress} variant="success" />
<ProgressBar value={0.4} color="linear-gradient(90deg, #22d3ee, #3b82f6)" />
```

| Prop      | Type                                             | Default    | Description                                         |
| --------- | ------------------------------------------------ | ---------- | --------------------------------------------------- |
| `value`   | `number`                                         | required   | Progress from `0` to `1`                            |
| `variant` | `'accent' \| 'success' \| 'warning' \| 'danger'` | `'accent'` | Colour variant                                      |
| `color`   | `string`                                         | —          | Custom CSS colour or gradient (overrides `variant`) |
| `class`   | `string`                                         | —          | Extra CSS classes                                   |

---

### SectionLabel

An uppercase, bold, small-text label. Used for grouping items within a panel or beside controls.

```svelte
<SectionLabel>Output Format</SectionLabel>
<SectionLabel class="w-20">Status</SectionLabel>
```

| Prop       | Type      | Default  | Description                                          |
| ---------- | --------- | -------- | ---------------------------------------------------- |
| `class`    | `string`  | —        | Extra CSS classes (useful for fixed widths in grids) |
| `children` | `Snippet` | required | Label text                                           |

---

### SegmentedControl

A button-group picker for a small fixed set of options. Animates the active indicator using the pill crossfade.

```svelte
<script lang="ts">
	let format = $state<'cbz' | 'epub'>('cbz');
</script>

<SegmentedControl
	label="Format"
	options={[
		{ value: 'cbz', label: 'CBZ' },
		{ value: 'epub', label: 'EPUB' }
	]}
	bind:value={format}
	onchange={(v) => console.log(v)}
/>
```

| Prop       | Type                            | Default  | Description                                    |
| ---------- | ------------------------------- | -------- | ---------------------------------------------- |
| `options`  | `{ value: T; label: string }[]` | required | Options array                                  |
| `value`    | `T`                             | required | Bindable selected value                        |
| `label`    | `string`                        | —        | Control label (rendered as `anasthasia-label`) |
| `disabled` | `boolean`                       | `false`  | Disables all segments                          |
| `class`    | `string`                        | —        | Extra CSS classes                              |
| `onchange` | `(value: T) => void`            | —        | Called when selection changes                  |

**Use when:** you have 2–5 mutually exclusive options with short labels. For more options or longer labels, use `Select`.

---

### Select

A styled dropdown select. The dropdown portals to `document.body`, so it always renders above other content including dialogs.

```svelte
<script lang="ts">
	let container = $state<'cbz' | 'epub' | 'raw'>('cbz');
</script>

<Select
	label="Container"
	hint="Choose the output format."
	options={[
		{ value: 'cbz', label: 'CBZ' },
		{ value: 'epub', label: 'EPUB' },
		{ value: 'raw', label: 'Raw' }
	]}
	bind:value={container}
/>
```

| Prop          | Type                            | Default              | Description                              |
| ------------- | ------------------------------- | -------------------- | ---------------------------------------- |
| `options`     | `{ value: T; label: string }[]` | required             | Options array                            |
| `value`       | `T`                             | required             | Bindable selected value                  |
| `label`       | `string`                        | —                    | Field label                              |
| `hint`        | `string`                        | —                    | Helper text (hidden when `error` is set) |
| `error`       | `string`                        | —                    | Error message; applies red border        |
| `placeholder` | `string`                        | `'Select an option'` | Shown when nothing is selected           |
| `disabled`    | `boolean`                       | `false`              | Disables the control                     |
| `class`       | `string`                        | —                    | Extra CSS classes on the wrapper         |
| `onchange`    | `(value: T) => void`            | —                    | Called when selection changes            |
| `icon`        | `Snippet`                       | —                    | Replaces the default chevron icon        |

**Keyboard:** `↓` / `↑` navigate options, `Enter` / `Space` open or confirm, `Escape` closes.

---

### ToastProvider

Renders the global toast notification stack. Place once at the root of your app. Reads from the `toast` store.

```svelte
<!-- In your layout -->
<ToastProvider />
```

No props. Trigger toasts from anywhere via the `toast` store:

```svelte
<script lang="ts">
	import { toast } from 'anasthasia';
</script>

<button onclick={() => toast.success('File saved.', { title: 'Saved' })}>Save</button>
<button onclick={() => toast.danger('Could not write file.', { title: 'Error' })}>Fail</button>
```

**toast API:**

```ts
toast.info(message: string, options?: ToastOptions): void
toast.success(message: string, options?: ToastOptions): void
toast.warning(message: string, options?: ToastOptions): void
toast.danger(message: string, options?: ToastOptions): void

interface ToastOptions {
  title?: string;
  actionLabel?: string;
  action?: () => void;
}
```

---

### Toggle

A boolean on/off switch.

```svelte
<Toggle label="Dark mode" bind:checked={dark} onchange={(v) => theme.toggle()} />
<Toggle label="Notifications" checked={false} disabled />
```

| Prop       | Type                         | Default  | Description             |
| ---------- | ---------------------------- | -------- | ----------------------- |
| `checked`  | `boolean`                    | required | Bindable toggle state   |
| `onchange` | `(checked: boolean) => void` | —        | Called when toggled     |
| `disabled` | `boolean`                    | `false`  | Disables the control    |
| `label`    | `string`                     | —        | Toggle label text       |
| `hint`     | `string`                     | —        | Helper text below label |
| `class`    | `string`                     | —        | Extra CSS classes       |

---

## Stores

### `theme`

Controls light/dark mode. Persists to `localStorage` under key `'anasthasia:theme'`.

```ts
import { theme } from 'anasthasia';

theme.init(); // Call once on mount — restores persisted preference
theme.toggle(); // Switch between light and dark
theme.dark; // boolean — true when dark mode is active
```

### `toast`

Global toast notification queue. See [ToastProvider](#toastprovider) for the full API.

### `uiPrefs`

Shared UI preferences. Currently exposes:

```ts
import { uiPrefs } from 'anasthasia';

uiPrefs.showKeyHints = true; // Show / hide the KeyHintBar
```

### `keyHint`

Registers keyboard shortcut hints displayed in `KeyHintBar`.

```ts
import { keyHint } from 'anasthasia';

// Returns an unregister function — call from onMount return
const unregister = keyHint.register([
	['meta+keys', 'Save'],
	['escape', 'Cancel']
]);
```

---

## Transitions

All transitions are tuned to match the design system's motion feel: fast (150ms), base (200ms), slow (300ms) with `cubicOut` easing.

```ts
import {
	riseIn,
	riseOut,
	pageFade,
	slideUp,
	slideDown,
	sidebarSlide,
	softCollapse,
	glassCollapse,
	sendPill,
	receivePill,
	duration,
	easing
} from 'anasthasia';
```

| Transition                 | Direction                  | Use for                                 |
| -------------------------- | -------------------------- | --------------------------------------- |
| `riseIn`                   | Rises up (y → 0) on enter  | Panels, dialogs, popovers, step content |
| `riseOut`                  | Drops down (0 → y) on exit | Symmetric exit for riseIn               |
| `pageFade`                 | Fade only                  | Page-level route transitions            |
| `slideUp`                  | Flies up from below        | Toasts, bottom drawers                  |
| `slideDown`                | Flies down from above      | Expanding top panels                    |
| `sidebarSlide`             | Slides on x-axis           | Sidebars, drawers                       |
| `softCollapse`             | Height + opacity           | Conditional form sections               |
| `glassCollapse`            | Width + blur + translate   | Inline chips, status tags               |
| `sendPill` / `receivePill` | Crossfade on position      | Active indicator (tabs, segments)       |

```svelte
<!-- Standard enter/exit -->
<div in:riseIn out:riseOut>...</div>

<!-- Conditional region collapse -->
{#if showAdvanced}
	<div transition:softCollapse>
		<!-- content -->
	</div>
{/if}

<!-- Pill crossfade between siblings -->
{#if active}
	<span in:receivePill={{ key }} out:sendPill={{ key }}></span>
{/if}
```

All transitions accept optional `{ duration?: number; y?: number }` params (where applicable).

---

## Utility Classes

Defined in `anasthasia/styles`, available globally.

| Class                   | Description                                                    |
| ----------------------- | -------------------------------------------------------------- |
| `.anasthasia-label`     | `text-xs font-bold uppercase tracking-wider text-muted`        |
| `.anasthasia-caption`   | `text-xs leading-relaxed text-muted`                           |
| `.anasthasia-body`      | `text-sm leading-relaxed text-muted`                           |
| `.anasthasia-mono`      | `font-mono text-xs text-text`                                  |
| `.text-accent-gradient` | Gradient text using `--background-image-accent-gradient`       |
| `.text-fill-reset`      | Resets `-webkit-text-fill-color` to `currentColor`             |
| `.bevel-accent`         | Accent-tinted box-shadow bevel — for elevated primary surfaces |

---

## Writing a Custom Flavour

A flavour is a plain CSS file you create **in your own app**. It overrides CSS custom properties on `:root` so that components adapt automatically without any class name changes.

Anasthasia's `styles.css` registers the `--color-anasthasia-*` namespace in its `@theme` block (build-time only). Your flavour sets the actual values at runtime via `:root {}` — later stylesheet wins.

### How it works

- **Colours** are plain `:root` custom properties using the `--color-anasthasia-*` names. Tailwind v4 utilities like `bg-anasthasia-surface` reference `var(--color-anasthasia-surface)` at runtime, so overriding the var is enough.
- **Design tokens** (radius, shadow) override Tailwind's own CSS variables (`--radius-lg`, `--shadow-md`, etc.) on `:root`. Components use standard Tailwind classes like `rounded-lg` — the flavour controls what those resolve to.
- **Dark mode** overrides use a `:root.dark {}` block. The higher specificity (0,2,0 vs 0,1,0) ensures the flavour always beats the base dark defaults regardless of stylesheet load order.

### Token reference

| Token                                                         | Type     | Role                                                       |
| ------------------------------------------------------------- | -------- | ---------------------------------------------------------- |
| `--color-anasthasia-bg`                                       | colour   | Page / window background                                   |
| `--color-anasthasia-surface`                                  | colour   | Primary content containers (cards, panels)                 |
| `--color-anasthasia-panel`                                    | colour   | Panel headers, sidebars, nested sections                   |
| `--color-anasthasia-border`                                   | colour   | Borders and dividers                                       |
| `--color-anasthasia-text`                                     | colour   | Primary body text                                          |
| `--color-anasthasia-muted`                                    | colour   | Secondary text, placeholders, hints                        |
| `--color-anasthasia-accent`                                   | colour   | Interactive colour — links, focus rings, highlights        |
| `--color-anasthasia-accent-strong`                            | colour   | Pressed / stronger variant of accent                       |
| `--background-image-accent-gradient`                          | gradient | Primary button background (dark) and gradient text (light) |
| `--radius-sm` / `--radius-md` / `--radius-lg` / `--radius-xl` | length   | Override Tailwind's `rounded-*` scale                      |
| `--shadow-sm` / `--shadow-md` / `--shadow-lg`                 | value    | Override Tailwind's `shadow-*` scale (optional)            |

### Surface depth order

The three surface colours must form a visible depth hierarchy:

```
--color-anasthasia-bg  <  --color-anasthasia-panel  <  --color-anasthasia-surface
```

In light mode `--bg` is a slight grey, `--surface` is pure white, `--panel` sits between. In dark mode the order reverses — `--bg` is deepest black, `--surface` is slightly lighter.

### Accent colour rules

- `--color-anasthasia-accent` must be readable as text on both `--surface` and `--panel`. Aim for WCAG AA (4.5:1).
- `--color-anasthasia-accent-strong` should be ~15–20% darker in light mode, slightly brighter in dark mode.
- `--background-image-accent-gradient` is a `linear-gradient(...)`. In light mode it renders as gradient text on a black button; in dark mode as the button background with black text. Both must be legible.

### Border opacity

Use semi-transparent values for `--color-anasthasia-border` so it adapts across surface depths:

```css
/* Light mode */
--color-anasthasia-border: rgba(0, 0, 0, 0.1);

/* Dark mode */
--color-anasthasia-border: rgba(255, 255, 255, 0.08);
```

### Full example

```css
/* src/styles/flavours/ocean.css */
:root {
	/* Colours */
	--color-anasthasia-bg: #f0f4f8;
	--color-anasthasia-surface: #ffffff;
	--color-anasthasia-panel: #e8eef4;
	--color-anasthasia-border: rgba(0, 0, 0, 0.08);
	--color-anasthasia-text: #0f172a;
	--color-anasthasia-muted: #64748b;
	--color-anasthasia-accent: #0ea5e9;
	--color-anasthasia-accent-strong: #0284c7;
	--background-image-accent-gradient: linear-gradient(
		135deg,
		#7dd3fc 0%,
		#0ea5e9 50%,
		#0284c7 100%
	);

	/* Radius — override Tailwind's built-in rounded-* scale */
	--radius-sm: 0.25rem;
	--radius-md: 0.375rem;
	--radius-lg: 0.5rem;
	--radius-xl: 0.75rem;
}

:root.dark {
	--color-anasthasia-bg: #020b14;
	--color-anasthasia-surface: #0d1f2d;
	--color-anasthasia-panel: #112233;
	--color-anasthasia-border: rgba(255, 255, 255, 0.07);
	--color-anasthasia-text: #f1f5f9;
	--color-anasthasia-muted: #94a3b8;
	--color-anasthasia-accent: #38bdf8;
	--color-anasthasia-accent-strong: #7dd3fc;
	--background-image-accent-gradient: linear-gradient(
		135deg,
		#7dd3fc 0%,
		#38bdf8 50%,
		#0ea5e9 100%
	);
}
```

### Wiring it up

Import the flavour **after** `anasthasia/styles` in your root stylesheet:

```css
@import 'tailwindcss';
@import 'anasthasia/styles';
@import './styles/flavours/ocean.css';
```

No component changes needed. Every Anasthasia component uses standard Tailwind classes (`rounded-lg`, `bg-anasthasia-surface`, etc.) — the flavour controls what those resolve to.

### Testing your flavour

Check each of the following in both light and dark mode:

- [ ] `--color-anasthasia-accent` text is readable on `--surface` and `--panel`
- [ ] `--color-anasthasia-muted` is clearly lower contrast than `--color-anasthasia-text`
- [ ] `--color-anasthasia-border` is visible against all three surface colours
- [ ] The primary `Button` renders correctly (gradient text in light, gradient background in dark)
- [ ] Focus rings are clearly visible on both light and dark backgrounds
- [ ] The `bevel-accent` shadow is visible on the primary button in dark mode
- [ ] `rounded-lg` / `rounded-xl` components reflect your intended radius scale
