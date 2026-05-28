import type { Component } from 'svelte';
import type { WizardStore } from './state.svelte';
import Step1Source from '$components/wizard/Step1Source.svelte';
import Step3Format from '$components/wizard/Step3Format.svelte';
import StepVolumeReview from '$components/wizard/StepVolumeReview.svelte';
import Step7PageEditor from '$components/wizard/Step7PageEditor.svelte';
import Step8Review from '$components/wizard/Step8Review.svelte';
import Step9Convert from '$components/wizard/Step9Convert.svelte';

export type WizardStep = {
    id: string;
    label: string;
    component: Component<any>;
    condition?: (w: InstanceType<typeof WizardStore>) => boolean;
    /** Step registers its own shift+arrowright handler — host should NOT bind a default one. */
    selfManagedNext?: boolean;
};

export const STEPS: WizardStep[] = [
    { id: 'source', label: 'Setup', component: Step1Source, selfManagedNext: true },
    { id: 'format', label: 'Output', component: Step3Format },
    {
        id: 'volume-review',
        label: 'Volumes',
        component: StepVolumeReview,
        condition: (w) => w.bundle !== 'flatten',
        selfManagedNext: true,
    },
    { id: 'page-editor', label: 'Pages', component: Step7PageEditor, selfManagedNext: true },
    { id: 'review', label: 'Review', component: Step8Review },
    { id: 'convert', label: 'Convert', component: Step9Convert },
];

export function activeSteps(w: InstanceType<typeof WizardStore>): WizardStep[] {
    return STEPS.filter((s) => !s.condition || s.condition(w));
}
