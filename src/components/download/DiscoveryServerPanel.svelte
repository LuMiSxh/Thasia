<script lang="ts">
    import { onDestroy, onMount } from 'svelte';
    import { downloadDir } from '@tauri-apps/api/path';
    import { open } from '@tauri-apps/plugin-dialog';
    import {
        Alert,
        Button,
        Panel,
        ProgressBar,
        Toggle,
        duration,
    } from 'anasthasia';
    import { slide } from 'svelte/transition';
    import {
        IconExternalLink,
        IconDownload,
        IconRefresh,
        IconTrash,
        IconPlayerPlay,
        IconPlayerStop,
        IconRotateClockwise,
        IconFolderOpen,
        IconPlus,
        IconX,
        IconClock,
    } from '@tabler/icons-svelte';
    import { commands, events, type DiscoverySettings, type RuntimeState } from '$types/bindings';
    import {
        DEFAULT_DISCOVERY_SETTINGS,
        loadDiscoverySettings,
        saveDiscoverySettings,
    } from '$lib/discovery-settings';

    let settings = $state<DiscoverySettings>({ ...DEFAULT_DISCOVERY_SETTINGS });
    let runtime = $state<RuntimeState>({ state: 'not_installed' });
    let installed = $state<{ version: string; size: number } | null>(null);
    let busy = $state('');
    let error = $state('');
    let progress = $state(0);
    let progressLabel = $state('');
    let updateInfo = $state('');
    let unlisteners: Array<() => void> = [];

    onMount(async () => {
        await refresh();
        unlisteners.push(
            await events.suwayomiStateChangedEvent.listen((event) => {
                runtime = event.payload.state;
                refreshInstalled();
            }),
            await events.suwayomiInstallProgressEvent.listen((event) => {
                const current = event.payload.progress;
                if (current.phase === 'downloading') {
                    progress = current.total ? current.bytes / current.total : 0.08;
                    progressLabel = `Downloading ${formatBytes(current.bytes)}${current.total ? ` / ${formatBytes(current.total)}` : ''}`;
                } else if (current.phase === 'verifying') {
                    progress = Math.max(progress, 0.99);
                    progressLabel = 'Verifying checksum';
                } else if (current.phase === 'extracting') {
                    progress = Math.max(progress, 0.995);
                    progressLabel = 'Extracting archive';
                } else {
                    progress = 1;
                    progressLabel = `Installed ${current.version}`;
                }
            })
        );
    });

    onDestroy(() => unlisteners.forEach((unlisten) => unlisten()));

    async function refresh() {
        settings = await loadDiscoverySettings();
        // suwayomiInstalledInfo and suwayomiStatus are independent — run in parallel.
        const [, statusResult] = await Promise.all([
            refreshInstalled(),
            commands.suwayomiStatus(),
        ]);
        if (installed && busy !== 'install') {
            progress = 0;
            progressLabel = '';
        }
        if (statusResult.status === 'ok') runtime = statusResult.data;
    }

    async function refreshInstalled() {
        const info = await commands.suwayomiInstalledInfo();
        installed = info.status === 'ok' ? info.data : null;
    }

    async function persist(next: DiscoverySettings) {
        settings = next;
        await saveDiscoverySettings(next);
    }

    async function run(
        label: string,
        action: () => Promise<{ status: 'ok'; data: unknown } | { status: 'error'; error: string }>
    ) {
        if (busy) return;
        busy = label;
        error = '';
        try {
            const result = await action();
            if (result.status === 'error') error = result.error;
            await refresh();
        } catch (e) {
            error = String(e);
        } finally {
            busy = '';
        }
    }

    async function install() {
        progress = 0;
        progressLabel = 'Preparing download';
        await run('install', () => commands.suwayomiInstall(null));
    }

    async function uninstall() {
        await run('delete', () => commands.suwayomiUninstall());
    }

    async function checkUpdate() {
        await run('check-update', async () => {
            const result = await commands.suwayomiCheckUpdate();
            if (result.status === 'ok') {
                updateInfo = result.data.available
                    ? `Update available: ${result.data.latest_version}`
                    : `Current release: ${result.data.latest_version}`;
            }
            return result;
        });
    }

    async function toggleEnabled(enabled: boolean) {
        await persist({ ...settings, enabled });
    }

    async function toggleAutoStart(autoStart: boolean) {
        await persist({ ...settings, autoStart });
    }

    async function updateRepo(index: number, value: string) {
        const extensionRepos = [...repoList()];
        extensionRepos[index] = value;
        await persist({ ...settings, extensionRepos });
    }

    async function addRepo() {
        await persist({ ...settings, extensionRepos: [...repoList(), ''] });
    }

    async function removeRepo(index: number) {
        const extensionRepos = repoList().filter((_, i) => i !== index);
        await persist({ ...settings, extensionRepos });
    }

    async function resetRepos() {
        await persist({ ...settings, extensionRepos: [...DEFAULT_DISCOVERY_SETTINGS.extensionRepos!] });
    }

    async function chooseDownloadDir() {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Choose Discovery download folder',
        });
        if (typeof selected === 'string') {
            await persist({ ...settings, downloadDir: selected });
        }
    }

    async function useSystemDownloads() {
        await persist({ ...settings, downloadDir: await downloadDir() });
    }

    async function useTemporaryDownloads() {
        await persist({ ...settings, downloadDir: null });
    }

    function statusText(state: RuntimeState): string {
        if (state.state === 'ready') return `Ready on ${state.port}`;
        if (state.state === 'starting') return 'Starting';
        if (state.state === 'not_running') return 'Not running';
        if (state.state === 'not_installed') return 'Not installed';
        return 'Error';
    }

    function statusClass(state: RuntimeState): string {
        if (state.state === 'ready')
            return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-500';
        if (state.state === 'starting') return 'border-amber-500/30 bg-amber-500/10 text-amber-500';
        if (state.state === 'error') return 'border-red-500/30 bg-red-500/10 text-red-400';
        return 'border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted';
    }

    function formatBytes(bytes: number): string {
        if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
        if (bytes < 1024 * 1024 * 1024) return `${Math.round(bytes / 1024 / 1024)} MB`;
        return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
    }

    function repoList(): string[] {
        return settings.extensionRepos ?? DEFAULT_DISCOVERY_SETTINGS.extensionRepos ?? [];
    }

    function formatLastChecked(raw: string | null | undefined): string {
        if (!raw) return '';
        const epoch = parseInt(raw, 10);
        if (isNaN(epoch)) return raw;
        return new Date(epoch * 1000).toLocaleString(undefined, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    }
</script>

<div class="flex flex-col gap-4">
    <Panel label="Integration" title="Suwayomi-Server">
        <p class="text-sm leading-6 text-anasthasia-muted">
            Discovery uses
            <a
                class="text-anasthasia-accent hover:underline"
                href="https://github.com/Suwayomi/Suwayomi-Server"
                target="_blank"
                rel="noreferrer"
            >
                Suwayomi-Server <IconExternalLink size={12} class="inline" />
            </a>
            for catalog sources and extension management.
        </p>
    </Panel>

    {#if error}
        <Alert variant="danger" title="Discovery error">{error}</Alert>
    {/if}

    <div class="grid gap-4 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] 2xl:grid-cols-4">
        <section class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
                <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                    <span
                        class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                        >Installation</span
                    >
                </div>
                <div class="flex flex-col gap-4 px-4 py-3">
                    <div class="flex items-start justify-between gap-4">
                        <div>
                            <div class="text-sm font-medium">
                                {installed ? `Installed ${installed.version}` : 'Not installed'}
                            </div>
                            <div class="mt-1 text-xs text-anasthasia-muted">
                                {installed
                                    ? `${formatBytes(installed.size)} in app data`
                                    : 'Install Suwayomi-Server to enable catalog discovery.'}
                            </div>
                            {#if settings.lastUpdateCheck || updateInfo}
                                <div class="mt-1 text-xs text-anasthasia-muted">
                                    {updateInfo || `Last checked ${formatLastChecked(settings.lastUpdateCheck)}`}
                                </div>
                            {/if}
                        </div>
                        <Toggle
                            checked={settings.enabled}
                            disabled={!installed}
                            onchange={toggleEnabled}
                            label="Enable Discovery"
                        />
                    </div>

                    {#if busy === 'install'}
                        <div
                            class="flex flex-col gap-2"
                            transition:slide={{ duration: duration.base }}
                        >
                            <div class="flex justify-between text-xs text-anasthasia-muted">
                                <span>{progressLabel}</span>
                                <span>{Math.round(progress * 100)}%</span>
                            </div>
                            <ProgressBar value={progress} class="h-1.5" />
                        </div>
                    {/if}

                    <div class="flex flex-wrap gap-2">
                        {#if !installed}
                            <Button
                                variant="primary"
                                loading={busy === 'install'}
                                loadingLabel="Installing…"
                                onclick={install}
                            >
                                <IconDownload size={15} /> Install Suwayomi-Server (~330 MB)
                            </Button>
                        {:else}
                            <Button
                                variant="secondary"
                                loading={busy === 'check-update'}
                                loadingLabel="Checking…"
                                onclick={checkUpdate}
                            >
                                <IconRefresh size={15} /> Check update
                            </Button>
                            <Button
                                variant="secondary"
                                loading={busy === 'install'}
                                loadingLabel="Reinstalling…"
                                onclick={install}
                            >
                                <IconDownload size={15} /> Reinstall
                            </Button>
                            <Button
                                variant="danger"
                                loading={busy === 'delete'}
                                loadingLabel="Deleting…"
                                onclick={uninstall}
                            >
                                <IconTrash size={15} /> Delete
                            </Button>
                        {/if}
                    </div>
                </div>
            </section>

            {#if installed}
                <section
                    class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface xl:col-span-1 2xl:col-span-2"
                >
                    <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                        <span
                            class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >Extension repositories</span
                        >
                    </div>
                    <div class="flex flex-col gap-3 px-4 py-3">
                        <div class="text-xs leading-5 text-anasthasia-muted">
                            Suwayomi no longer ships default extension repositories. Thasia starts
                            with Keiyoushi, a community-maintained Mihon extension repository.
                            Changes apply after restart.
                        </div>
                        <div class="flex flex-col gap-2">
                            {#each repoList() as repo, i (i)}
                                <div class="flex gap-2">
                                    <input
                                        class="h-9 min-w-0 flex-1 rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 font-mono text-xs text-anasthasia-text transition-colors duration-150 outline-none placeholder:text-anasthasia-muted hover:border-anasthasia-accent/40 focus:border-anasthasia-accent/60"
                                        placeholder="https://raw.githubusercontent.com/user/repo/index.min.json"
                                        value={repo}
                                        oninput={(event) =>
                                            updateRepo(i, event.currentTarget.value)}
                                    />
                                    <button
                                        onclick={() => removeRepo(i)}
                                        aria-label="Remove extension repository"
                                        class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted transition-colors duration-150 hover:border-red-500/40 hover:text-red-400"
                                    >
                                        <IconX size={14} />
                                    </button>
                                </div>
                            {/each}
                        </div>
                        <div class="flex flex-wrap gap-2">
                            <Button variant="secondary" size="sm" onclick={addRepo}>
                                <IconPlus size={14} /> Add repo
                            </Button>
                            <Button variant="ghost" size="sm" onclick={resetRepos}>
                                <IconRefresh size={14} /> Reset default
                            </Button>
                        </div>
                    </div>
                </section>

                <section
                    class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                >
                    <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                        <span
                            class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >Runtime</span
                        >
                    </div>
                    <div class="flex flex-col gap-4 px-4 py-3">
                        <div class="flex items-center justify-between gap-4">
                            <Toggle
                                checked={settings.autoStart}
                                onchange={toggleAutoStart}
                                label="Start with Thasia"
                            />
                            <span
                                class="inline-flex items-center rounded-md border px-2 py-1 text-xs font-bold {statusClass(
                                    runtime
                                )}"
                            >
                                {statusText(runtime)}
                            </span>
                        </div>
                        {#if runtime.state === 'error'}
                            <Alert variant="danger" title="Runtime failed">{runtime.message}</Alert>
                        {/if}
                        <div class="flex flex-wrap gap-2">
                            <Button
                                variant="primary"
                                loading={busy === 'start'}
                                loadingLabel="Starting…"
                                disabled={runtime.state === 'ready'}
                                onclick={() => run('start', () => commands.suwayomiStart())}
                            >
                                <IconPlayerPlay size={15} /> Start
                            </Button>
                            <Button
                                variant="secondary"
                                loading={busy === 'stop'}
                                loadingLabel="Stopping…"
                                disabled={runtime.state !== 'ready'}
                                onclick={() => run('stop', () => commands.suwayomiStop())}
                            >
                                <IconPlayerStop size={15} /> Stop
                            </Button>
                            <Button
                                variant="secondary"
                                loading={busy === 'restart'}
                                loadingLabel="Restarting…"
                                onclick={() => run('restart', () => commands.suwayomiRestart())}
                            >
                                <IconRotateClockwise size={15} /> Restart
                            </Button>
                        </div>
                    </div>
                </section>

                <section
                    class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                >
                    <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                        <span
                            class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >Downloads</span
                        >
                    </div>
                    <div class="flex flex-col gap-3 px-4 py-3">
                        <div class="flex items-start justify-between gap-4">
                            <div class="min-w-0 flex-1">
                                <div class="text-sm font-medium">Download location</div>
                                <div class="mt-1 text-xs text-anasthasia-muted">
                                    Downloaded chapters are stored as CBZ files in a per-series
                                    folder.
                                </div>
                            </div>
                            <span
                                class="rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-1 text-xs font-bold text-anasthasia-muted"
                            >
                                {settings.downloadDir ? 'Custom' : 'Temporary'}
                            </span>
                        </div>

                        <div
                            class="flex h-9 min-w-0 items-center rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 font-mono text-xs {settings.downloadDir
                                ? 'text-anasthasia-text'
                                : 'text-anasthasia-muted'}"
                            title={settings.downloadDir ?? undefined}
                        >
                            <span class="truncate">{settings.downloadDir ?? 'Temporary folder'}</span>
                        </div>

                        <div class="flex flex-wrap gap-2">
                            <Button variant="secondary" size="sm" onclick={chooseDownloadDir}>
                                <IconFolderOpen size={14} /> Choose folder
                            </Button>
                            <Button variant="secondary" size="sm" onclick={useSystemDownloads}>
                                <IconDownload size={14} /> Use Downloads
                            </Button>
                            <Button variant="ghost" size="sm" onclick={useTemporaryDownloads}>
                                <IconClock size={14} /> Use temporary
                            </Button>
                        </div>
                    </div>
                </section>

            {/if}
    </div>
</div>
