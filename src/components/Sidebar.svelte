<script lang="ts">
    import { sidebarSlide } from '$lib/transitions';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { theme } from '$lib/theme.svelte';
    import { STEPS } from '$lib/wizard/steps';
    import { page } from '$app/stores';
    import {
        IconHome,
        IconFileSignal,
        IconSettings,
        IconChevronRight,
        IconChevronLeft,
        IconCheck,
        IconSun,
        IconMoon,
    } from '@tabler/icons-svelte';

    let sidebarSteps = $derived(
        STEPS.map((s) => ({
            id: s.id,
            label: s.label,
            status: (wizard.completedStepIds.has(s.id)
                ? 'done'
                : s.id === wizard.currentStepId
                  ? 'active'
                  : s.condition && !s.condition(wizard)
                    ? 'conditional'
                    : 'locked') as 'done' | 'active' | 'locked' | 'conditional',
        }))
    );

    function handleStepClick(id: string, status: string) {
        if (status === 'done') {
            document.dispatchEvent(new CustomEvent('wizard:goto', { detail: id }));
        }
    }
</script>

<!-- Sidebar container: tab always visible, panel slides in beside it -->
<aside class="z-50 flex h-full flex-shrink-0">
    {#if sidebar.isOpen}
        <!-- Panel -->
        <nav
            class="flex w-52 flex-col overflow-hidden border-r border-thasia-border bg-thasia-surface"
            transition:sidebarSlide
        >
            <!-- Wordmark -->
            <a
                href="/"
                class="block flex-shrink-0 border-b border-thasia-border px-4 pt-5 pb-4 transition-colors duration-150 hover:bg-thasia-panel"
            >
                <div class="text-sm font-bold tracking-widest text-thasia-accent uppercase">
                    Thasia
                </div>
                <div class="mt-0.5 text-[10px] tracking-wider text-thasia-muted uppercase">
                    Engine
                </div>
            </a>

            <!-- Content -->
            <div class="flex flex-1 flex-col gap-1 overflow-y-auto p-3">
                {#if sidebar.mode === 'nav'}
                    <a
                        href="/"
                        class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors duration-150
                               {$page.url.pathname === '/'
                            ? 'border border-thasia-accent/30 bg-thasia-accent/10 text-thasia-accent'
                            : 'text-thasia-muted hover:bg-thasia-panel hover:text-thasia-text'}"
                    >
                        <IconHome size={15} />
                        Home
                    </a>
                    <a
                        href="/convert"
                        class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors duration-150
                               {$page.url.pathname.startsWith('/convert')
                            ? 'border border-thasia-accent/30 bg-thasia-accent/10 text-thasia-accent'
                            : 'text-thasia-muted hover:bg-thasia-panel hover:text-thasia-text'}"
                    >
                        <IconFileSignal size={15} />
                        Convert
                    </a>
                    <a
                        href="/settings"
                        class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors duration-150
                               {$page.url.pathname.startsWith('/settings')
                            ? 'border border-thasia-accent/30 bg-thasia-accent/10 text-thasia-accent'
                            : 'text-thasia-muted hover:bg-thasia-panel hover:text-thasia-text'}"
                    >
                        <IconSettings size={15} />
                        Settings
                    </a>
                {:else}
                    <!-- Exit wizard -->
                    <a
                        href="/"
                        class="mb-1 flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm
                               text-thasia-muted transition-colors duration-150 hover:bg-thasia-panel hover:text-thasia-text"
                    >
                        <IconHome size={15} />
                        Home
                    </a>
                    <!-- Wizard steps -->
                    <div
                        class="mb-1 px-2 text-xs font-bold tracking-wider text-thasia-muted uppercase"
                    >
                        Wizard
                    </div>
                    {#each sidebarSteps as step}
                        <button
                            onclick={() => handleStepClick(step.id, step.status)}
                            disabled={step.status === 'locked' || step.status === 'conditional'}
                            class="
                flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left text-sm
                transition-colors duration-150
                {step.status === 'active'
                                ? 'border border-thasia-accent/30 bg-thasia-accent/10 text-thasia-text'
                                : step.status === 'done'
                                  ? 'cursor-pointer text-thasia-muted hover:bg-thasia-panel hover:text-thasia-text'
                                  : 'cursor-default text-thasia-muted opacity-50'}
              "
                        >
                            <!-- Step indicator dot -->
                            <span
                                class="
                flex h-4 w-4 flex-shrink-0 items-center justify-center rounded-full
                {step.status === 'done'
                                    ? 'bg-thasia-accent'
                                    : step.status === 'active'
                                      ? 'border-2 border-thasia-accent'
                                      : step.status === 'conditional'
                                        ? 'border border-dashed border-thasia-border'
                                        : 'border border-thasia-border'}
              "
                            >
                                {#if step.status === 'done'}
                                    <IconCheck
                                        size={10}
                                        class="text-black dark:text-zinc-900"
                                        stroke={3}
                                    />
                                {/if}
                            </span>
                            <span class="truncate {step.status === 'conditional' ? 'italic' : ''}"
                                >{step.label}</span
                            >
                        </button>
                    {/each}
                {/if}
            </div>

            <!-- Bottom: theme toggle -->
            <div class="flex-shrink-0 border-t border-thasia-border p-3">
                <button
                    onclick={() => theme.toggle()}
                    class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-thasia-muted
                 transition-colors duration-150 hover:bg-thasia-panel hover:text-thasia-text"
                >
                    {#if theme.dark}
                        <IconSun size={15} />
                        <span>Light mode</span>
                    {:else}
                        <IconMoon size={15} />
                        <span>Dark mode</span>
                    {/if}
                </button>
            </div>
        </nav>
    {/if}

    <!-- Tab — glued to right edge of panel, always rendered -->
    <button
        onclick={() => sidebar.toggle()}
        aria-label="Toggle sidebar"
        class="
      flex w-5 flex-shrink-0 items-center justify-center self-stretch
      border-r border-thasia-border bg-thasia-surface
      text-thasia-muted transition-colors duration-150
      hover:bg-thasia-panel hover:text-thasia-accent
    "
    >
        {#if sidebar.isOpen}
            <IconChevronLeft size={13} />
        {:else}
            <IconChevronRight size={13} />
        {/if}
    </button>
</aside>
