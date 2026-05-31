import type { ConvertOptions, VolumeEdit as BackendVolumeEdit } from '$types/bindings';
import type { WizardStore } from './state.svelte';

const IMAGE_FORMAT = { avif: 'Avif', webp: 'Webp', original: 'Original' } as const;
const OUTPUT_FORMAT = { cbz: 'Cbz', epub: 'Epub', raw: 'Raw' } as const;
const DIRECTION = { ltr: 'Ltr', rtl: 'Rtl' } as const;
const COLOR_ENHANCE = {
    off: 'Off',
    mild: 'Mild',
    balanced: 'Balanced',
    strong: 'Strong',
} as const;
const SHARPEN = { off: 'Off', mild: 'Mild' } as const;

export function buildConvertOptions(wizard: WizardStore): ConvertOptions {
    return {
        output_dir: wizard.outputDir,
        output_name: wizard.outputName,
        create_directory: wizard.createDirectory,
        image_format: IMAGE_FORMAT[wizard.imageFormat],
        max_width: wizard.maxWidth,
        force_reencode: wizard.forceReencode,
        clean_tones: wizard.cleanTones,
        color_enhance: COLOR_ENHANCE[wizard.colorEnhance],
        sharpen: SHARPEN[wizard.sharpen],
        output_format: OUTPUT_FORMAT[wizard.container],
        direction: DIRECTION[wizard.direction],
        bundle: wizard.bundle,
        volume_separator: wizard.volumeSeparator,
        hide_single_volume: wizard.hideSingleVolume,
    };
}

export function buildVolumeEdits(wizard: WizardStore): BackendVolumeEdit[] {
    return wizard.pageEdits.map((vol) => ({
        volume_num: vol.volumeNum,
        pages: vol.pages.map((p) => ({
            source:
                p.customPath !== null
                    ? { kind: 'custom' as const, path: p.customPath }
                    : {
                          kind: 'original' as const,
                          page_index: p.originalPageIndex ?? 0,
                          source_volume_num: p.sourceVolumeNum,
                      },
            excluded: p.excluded,
        })),
    }));
}
