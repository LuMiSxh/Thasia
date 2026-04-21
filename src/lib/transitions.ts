import { crossfade, fade, fly, scale, slide } from 'svelte/transition';
import { cubicOut } from 'svelte/easing';

/** Durations aligned with the design system (fast desktop feel) */
export const duration = {
    fast: 150,
    base: 200,
    slow: 300,
} as const;

/**
 * Sliding pill crossfade — use [sendPill, receivePill] for any
 * "active indicator moves between options" pattern (segmented controls,
 * tab bars, step indicators, etc.)
 *
 * Usage:
 *   {#if active}
 *     <span in:receivePill={{ key }} out:sendPill={{ key }}></span>
 *   {/if}
 */
export const [sendPill, receivePill] = crossfade({
    duration: duration.base,
    easing: cubicOut,
    fallback: (node) => scale(node, { duration: duration.fast, easing: cubicOut }),
});

/** Sidebar panel slide in/out on x-axis */
export const sidebarSlide = (node: Element) =>
    slide(node, { axis: 'x', duration: duration.base, easing: cubicOut });

/** Standard page-level fade */
export const pageFade = (node: Element) =>
    fade(node, { duration: duration.base, easing: cubicOut });

/** Slide up into view — good for toasts, dropdowns, step content */
export const slideUp = (node: Element, params?: { duration?: number }) =>
    fly(node, { y: 8, duration: params?.duration ?? duration.base, easing: cubicOut });

/** Slide down — good for panels expanding downward */
export const slideDown = (node: Element, params?: { duration?: number }) =>
    fly(node, { y: -8, duration: params?.duration ?? duration.base, easing: cubicOut });
