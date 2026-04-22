import type { Component } from 'svelte';
import type { WizardStore } from './state.svelte';
import Step1Source from '$components/wizard/Step1Source.svelte';
import Step2Destination from '$components/wizard/Step2Destination.svelte';
import Step3ImageFormat from '$components/wizard/Step3ImageFormat.svelte';
import Step4Container from '$components/wizard/Step4Container.svelte';
import Step5Direction from '$components/wizard/Step5Direction.svelte';
import Step6Bundling from '$components/wizard/Step6Bundling.svelte';
import StepVolumeReview from '$components/wizard/StepVolumeReview.svelte';
import Step7PageEditor from '$components/wizard/Step7PageEditor.svelte';
import Step8Review from '$components/wizard/Step8Review.svelte';
import Step9Convert from '$components/wizard/Step9Convert.svelte';

export type WizardStep = {
    id: string;
    label: string;

    component: Component<any>;
    condition?: (w: InstanceType<typeof WizardStore>) => boolean;
};

export const STEPS: WizardStep[] = [
    { id: 'source', label: 'Source', component: Step1Source },
    { id: 'destination', label: 'Destination', component: Step2Destination },
    { id: 'image-format', label: 'Image Format', component: Step3ImageFormat },
    { id: 'container', label: 'Container', component: Step4Container },
    {
        id: 'direction',
        label: 'Direction',
        component: Step5Direction,
        condition: (w) => w.container === 'epub',
    },
    { id: 'bundling', label: 'Bundling', component: Step6Bundling },
    {
        id: 'volume-review',
        label: 'Volumes',
        component: StepVolumeReview,
        condition: (w) => w.bundle !== 'flatten',
    },
    { id: 'page-editor', label: 'Pages', component: Step7PageEditor },
    { id: 'review', label: 'Review', component: Step8Review },
    { id: 'convert', label: 'Convert', component: Step9Convert },
];

export function activeSteps(w: InstanceType<typeof WizardStore>): WizardStep[] {
    return STEPS.filter((s) => !s.condition || s.condition(w));
}
