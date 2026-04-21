# Thasia Engine — UI/UX Design Guidelines

## 1. Typography & Fonts

To make the app feel incredibly premium, fast, and native, we will use fonts that are highly legible at small sizes but have a modern, "machined" aesthetic.

**The Font Stack:**

- **Primary (UI & Body): [Geist Sans](https://vercel.com/font/sans)**
    - _Why:_ Created by Vercel. It is the current gold standard for modern developer tools. It is cleaner than Inter and looks incredibly sharp on high-density displays.
- **Secondary (Data & Technical): [JetBrains Mono](https://fonts.google.com/specimen/JetBrains+Mono)**
    - _Why:_ The undisputed king of modern monospace. It pairs beautifully with Geist.

### Typesetting Rules

- **Section Headers/Labels:** Use `text-xs font-bold tracking-wider uppercase text-thasia-muted`. (e.g., `DETECTION SETTINGS`).
- **Numbers & Data:** Any file size, chapter count, volume index, or file path MUST use the monospace font (`font-mono`).
- **Keyboard Hints:** Wrap in `<kbd>` tags, use `font-mono text-[10px] uppercase border`.

---

## 2. Color & Theming Strategy

You now have a semantic variable system (`bg-thasia-bg`, `text-thasia-accent`, etc.). **Never hardcode hex colors in your components.**

- **`thasia-bg`**: The absolute lowest layer. Use for the app shell background and deep inset wells (like inputs).
- **`thasia-surface`**: The primary card/panel layer. Use for Bento grid items, sidebars, and headers.
- **`thasia-panel`**: A subtle highlight layer. Use for table headers, footers, or hovering over normal items.
- **`thasia-border`**: The standard 1px line separating elements.
- **`thasia-accent`**: Metallic Gold. Use sparingly. If a screen has more than two massive gold elements, it is too loud.

---

## 3. Interactive Elements (Buttons & Inputs)

### Primary Buttons (The "Call to Action")

Primary buttons flip completely between light and dark mode to maintain luxury contrast.

- **Base Style:** `px-6 py-2 rounded-lg text-sm font-bold shadow-md active:translate-y-[1px] transition-all`
- **Light Mode:** Deep Charcoal background with Metallic Gold text.
- **Dark Mode:** Metallic Gold background with Black text.
- **Lighting:** In dark mode, apply the `.bevel-dark` custom utility.

**Implementation Note:** In Tailwind, apply the Light Mode styles as the base classes (no prefix). Then, override them with the `dark:` prefix for Dark Mode. This is the correct pattern.

**Example Class String:**

```html
<button
    class="dark:bevel-dark bg-zinc-900 text-thasia-accent hover:bg-zinc-800 dark:bg-gold-metallic dark:text-black dark:hover:brightness-110 ..."
>
    Confirm
</button>
```

### Secondary Buttons & Selectable Items

Items that can be clicked (like selecting a volume or a secondary action) rely on border highlights, not background color fills.

- **Base (Idle):** `bg-thasia-bg border border-thasia-border rounded-lg`
- **Hover State:** `hover:border-thasia-accent/50 transition-colors`
- **Active/Selected State:** `border-thasia-accent bg-white dark:bg-thasia-accent/10 shadow-sm`. (Never use a solid gold background for a list item).

### Inputs & Textareas

Inputs should look like physical depressions cut into the surface.

- **Style:** `bg-zinc-50 dark:bg-thasia-bg border border-zinc-300 dark:border-thasia-border rounded-lg px-3 py-1.5 shadow-inner`.
- **Focus State:** Drop the default browser outline. Use `outline-none focus:ring-1 ring-zinc-900 dark:ring-thasia-accent`.

---

## 4. Surfaces & Layout

We are abandoning "Apple Glass" (heavy backdrop blurs) in favor of "Tactile Hardware".

- **Border Radius:** Standardize on `rounded-xl` for large panels (Bento items), `rounded-lg` for lists/buttons, and `rounded-md` for small tags/badges. Do not use full pills (`rounded-full`) unless it is an actual circle (like an avatar).
- **Borders:** Everything should have a 1px border. Panels shouldn't "float" via drop shadows alone; they must be grounded by `border-thasia-border`.
- **Spacing:** Rely on flex/grid gaps. Use `gap-4` or `gap-6` between major panels, and `gap-2` between tightly related items (like an icon and text).

---

## 5. Motion & Accessibility

- **Speed:** Desktop apps should feel instantaneous. Keep transitions fast. Use `duration-150` or `duration-200`. Do not use `duration-500` unless it is a major page transition.
- **Active States:** Every clickable element must have an `active:` state. Usually `active:translate-y-[1px] active:shadow-none`. This physical "click" feedback is crucial for a premium feel.
- **Focus Rings:** For accessibility (tab navigation), ensure custom focus rings are visible. If a user tabs to a button, it should look focused: `focus-visible:ring-2 focus-visible:ring-thasia-accent focus-visible:ring-offset-2 focus-visible:ring-offset-thasia-surface`.
