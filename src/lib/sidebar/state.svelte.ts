export type SidebarMode = 'nav' | 'wizard';

export class SidebarStore {
    isOpen = $state(false);
    mode = $state<SidebarMode>('nav');

    open() {
        this.isOpen = true;
    }
    close() {
        this.isOpen = false;
    }
    toggle() {
        this.isOpen = !this.isOpen;
    }

    enterWizard() {
        this.mode = 'wizard';
        this.isOpen = true;
    }

    exitWizard() {
        this.mode = 'nav';
    }

    collapseForConversion() {
        this.isOpen = false;
    }
}

export const sidebar = new SidebarStore();
