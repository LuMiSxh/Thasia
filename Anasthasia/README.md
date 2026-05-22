<div align="center">

# Anasthasia

**A Svelte 5 component library and design system for desktop-grade applications**

Reusable UI primitives, design tokens, flavour theming, and interaction utilities — extracted from Thasia and built to scale.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/LuMiSxh/Anasthasia)](https://github.com/LuMiSxh/Anasthasia/tags)
[![Svelte](https://img.shields.io/badge/svelte-5-orange.svg)](https://svelte.dev)

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Development](#development) • [Component Reference](COMPONENTS.md)

</div>

---

## Features

### Design Tokens

A strict set of semantic CSS variables (`--bg`, `--surface`, `--panel`, `--border`, `--text`, `--muted`, `--accent`, `--accent-strong`) mapped to Tailwind v4 utility classes via `@theme`. Every component is built exclusively against these tokens — no hardcoded colors anywhere.

### Component Library

18 production-ready Svelte 5 components covering the core UI surface:

| Primitives         | Feedback        | Layout         | Keyboard          |
| ------------------ | --------------- | -------------- | ----------------- |
| `Button`           | `Alert`         | `Card`         | `Kbd`             |
| `Input`            | `Badge`         | `Panel`        | `KeyComboDisplay` |
| `Select`           | `ToastProvider` | `FieldRow`     | `KeyHintBar`      |
| `Toggle`           | `ProgressBar`   | `SectionLabel` | —                 |
| `SegmentedControl` | `Dialog`        | `PathDisplay`  | —                 |

### Theming

Light and dark mode via a `.dark` class on `<html>`. The `theme` store manages the toggle and persists the preference in localStorage, with OS-preference fallback. Switching is instant — all color transitions are CSS-driven at 300ms.

### Flavour System

A flavour is a single CSS file that defines both the light (`:root`) and dark (`.dark`) color tokens for a complete visual personality. `styles.css` ships no colors — a flavour import is required.

**Available flavours:**

| Flavour    | Light                                       | Dark                                     |
| ---------- | ------------------------------------------- | ---------------------------------------- |
| `imperial` | Luxury Cathedral — warm whites and gold     | Immortal Abyssal — deep blacks and gold  |
| `crimson`  | Crimson Dawn — warm off-whites and deep red | Crimson Noir — near-black with vivid red |

Import a flavour after `anasthasia/styles` in your stylesheet. Additional flavours can be added by creating a new CSS file with `:root` and `.dark` blocks.

### Transitions

11 named transition utilities tuned for a fast, desktop-native feel: `riseIn`, `riseOut`, `pageFade`, `slideUp`, `slideDown`, `sidebarSlide`, `softCollapse`, `glassCollapse`, `sendPill`, `receivePill`.

---

## Installation

```sh
pnpm add github:LuMiSxh/Anasthasia#v0.1.0
```

### Prerequisites

- [Svelte](https://svelte.dev) 5+
- [Tailwind CSS](https://tailwindcss.com) v4

---

## Usage

### 1. Bootstrap fonts

Import once in the root layout — loads Geist Sans and JetBrains Mono from `@fontsource`:

```svelte
<script lang="ts">
	import 'anasthasia/bootstrap';
	import './app.css';

	const { children } = $props();
</script>

{@render children()}
```

### 2. Import base styles and a flavour

In your root stylesheet:

```css
@import 'tailwindcss';
@import 'anasthasia/styles';
@import 'anasthasia/flavours/imperial';
```

`anasthasia/styles` provides the Tailwind theme tokens, typography rules, and utility classes. The flavour provides all color values. Both imports are required.

### 3. Use components

```svelte
<script lang="ts">
	import { Button, Card, Input, Select, Dialog } from 'anasthasia';
</script>

<Card>
	<Input label="Name" placeholder="Enter your name" />
	<Select
		label="Role"
		options={[
			{ value: 'admin', label: 'Admin' },
			{ value: 'user', label: 'User' }
		]}
		bind:value={role}
	/>
	<Button variant="primary">Save</Button>
</Card>
```

### 4. Theme toggle

```svelte
<script lang="ts">
	import { theme } from 'anasthasia';
</script>

<button onclick={() => theme.toggle()}>
	{theme.current === 'dark' ? 'Light mode' : 'Dark mode'}
</button>
```

### Available exports

| Export                         | Contents                                                                                                                         |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `anasthasia`                   | All 18 components, stores (`theme`, `toast`, `uiPrefs`, `keyHint`), `keyboard` manager, all transitions, `dropdownPortal` action |
| `anasthasia/bootstrap`         | Font imports (Geist Sans, JetBrains Mono)                                                                                        |
| `anasthasia/styles`            | Tailwind v4 theme tokens, typography, utilities — no colors                                                                      |
| `anasthasia/flavours/imperial` | Imperial flavour (light + dark color tokens)                                                                                     |

---

## Development

### Prerequisites

- [Node.js](https://nodejs.org) 22+
- [pnpm](https://pnpm.io)

### Setup

```sh
# Clone the repository
git clone https://github.com/LuMiSxh/Anasthasia.git
cd Anasthasia

# Install dependencies
pnpm install

# Run dev server
pnpm run dev

# Type check
pnpm run check

# Build package
pnpm run build
```

### Adding a flavour

Create `src/lib/flavours/<name>.css` with `:root` and `.dark` blocks using the eight token variables, then add an export entry to `package.json`:

```json
"./flavours/<name>": "./dist/flavours/<name>.css"
```

### Versioning

The `Version Tag` workflow runs lint, type-check, and build, then commits the bumped version and creates a `vX.Y.Z` git tag. Trigger it manually via GitHub Actions with a semver input.

---

## Component Reference

Full API documentation, design guidelines, and usage examples for all 18 components, stores, transitions, and utility classes are in [COMPONENTS.md](COMPONENTS.md).

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">

**Made with passion by LuMiSxh**

[GitHub](https://github.com/LuMiSxh/Anasthasia) • [Issues](https://github.com/LuMiSxh/Anasthasia/issues) • [Releases](https://github.com/LuMiSxh/Anasthasia/tags)

</div>
