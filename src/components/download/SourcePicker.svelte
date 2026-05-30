<script lang="ts">
    import { Button, Dialog, Input, Select } from 'anasthasia';
    import { commands, type ExtensionInfo, type SourceInfo } from '$types/bindings';
    import { IconDownload, IconPackage, IconRefresh, IconSearch } from '@tabler/icons-svelte';

    let {
        sources,
        selectedSourceId,
        onSelect,
        onRefresh,
    }: {
        sources: SourceInfo[];
        selectedSourceId: string;
        onSelect: (id: string) => void;
        onRefresh: () => Promise<void>;
    } = $props();

    let extensions = $state<ExtensionInfo[]>([]);
    let modalOpen = $state(false);
    let loading = $state(false);
    let busyPkg = $state('');
    let error = $state('');
    let extensionQuery = $state('');

    let sourceOptions = $derived(
        sources.map((source) => ({
            value: source.id,
            label: `${source.name}${source.lang ? ` · ${source.lang}` : ''}`,
        }))
    );
    let filteredExtensions = $derived.by(() => {
        const query = extensionQuery.trim().toLowerCase();
        if (!query) return extensions;
        return extensions.filter((ext) =>
            [ext.name, ext.pkg_name, ext.lang, ext.version_name]
                .filter(Boolean)
                .some((value) => value!.toLowerCase().includes(query))
        );
    });

    async function loadExtensions() {
        loading = true;
        error = '';
        const result = await commands.listAvailableExtensions();
        if (result.status === 'ok') extensions = result.data;
        else error = result.error;
        loading = false;
    }

    async function openManager() {
        modalOpen = true;
        extensionQuery = '';
        await loadExtensions();
    }

    async function toggleExtension(ext: ExtensionInfo) {
        busyPkg = ext.pkg_name;
        const result = ext.installed
            ? await commands.uninstallExtension(ext.pkg_name)
            : await commands.installExtension(ext.pkg_name);
        if (result.status === 'error') error = result.error;
        await loadExtensions();
        await onRefresh();
        busyPkg = '';
    }
</script>

<div class="grid gap-3 md:grid-cols-[minmax(16rem,1fr)_auto]">
    <Select
        label="Source"
        placeholder="Install a source extension first"
        value={selectedSourceId}
        options={sourceOptions}
        search
        disabled={sourceOptions.length === 0}
        error={sourceOptions.length === 0 ? 'No installed source extensions' : ''}
        onchange={onSelect}
    />
    <div class="flex flex-col gap-1.5">
        <span aria-hidden="true" class="anasthasia-label invisible">Source</span>
        <div class="flex gap-3">
            <Button variant="secondary" onclick={openManager}
                ><IconPackage size={15} /> Manage extensions</Button
            >
            <Button variant="ghost" onclick={onRefresh}><IconRefresh size={15} /> Refresh</Button>
        </div>
        <span aria-hidden="true" class="invisible text-xs">No installed source extensions</span>
    </div>
</div>

<Dialog
    open={modalOpen}
    title="Extensions"
    description="Install source extensions through Suwayomi-Server."
    class="!max-w-3xl"
    onclose={() => (modalOpen = false)}
>
    <div class="flex flex-col gap-3">
        {#if error}
            <div
                class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-400"
            >
                {error}
            </div>
        {/if}
        <Input
            bind:value={extensionQuery}
            placeholder="Search extensions"
            aria-label="Search extensions"
        />
        <div class="h-[52vh] min-h-80 overflow-y-auto">
            {#if loading}
                <div class="flex h-full items-center justify-center text-sm text-anasthasia-muted">
                    Loading extensions…
                </div>
            {:else if filteredExtensions.length > 0}
                <div class="flex flex-col divide-y divide-anasthasia-border">
                    {#each filteredExtensions as ext (ext.pkg_name)}
                        <div class="flex items-center justify-between gap-4 py-3">
                            <div class="min-w-0">
                                <div class="truncate text-sm font-medium">{ext.name}</div>
                                <div class="truncate text-xs text-anasthasia-muted">
                                    {ext.lang || 'unknown'} · {ext.version_name || ext.pkg_name}
                                </div>
                            </div>
                            <Button
                                size="sm"
                                variant={ext.installed ? 'secondary' : 'primary'}
                                loading={busyPkg === ext.pkg_name}
                                loadingLabel={ext.installed ? 'Removing…' : 'Installing…'}
                                onclick={() => toggleExtension(ext)}
                            >
                                <IconDownload size={14} />
                                {ext.installed ? 'Remove' : 'Install'}
                            </Button>
                        </div>
                    {/each}
                </div>
            {:else if extensions.length > 0}
                <div
                    class="flex h-full items-center justify-center rounded-lg border border-dashed border-anasthasia-border px-4 py-8 text-center"
                >
                    <div>
                        <div class="flex justify-center text-anasthasia-muted">
                            <IconSearch size={18} />
                        </div>
                        <div class="mt-2 text-sm font-medium">No matching extensions</div>
                        <div class="mt-1 text-xs text-anasthasia-muted">
                            Try a different name, package, or language.
                        </div>
                    </div>
                </div>
            {:else}
                <div
                    class="flex h-full items-center justify-center rounded-lg border border-dashed border-anasthasia-border px-4 py-8 text-center"
                >
                    <div>
                        <div class="text-sm font-medium">No extensions available</div>
                        <div class="mt-1 text-xs text-anasthasia-muted">
                            Check the Suwayomi tab, then refresh. Thasia should ship with the
                            Keiyoushi repo configured.
                        </div>
                    </div>
                </div>
            {/if}
        </div>
    </div>
</Dialog>
