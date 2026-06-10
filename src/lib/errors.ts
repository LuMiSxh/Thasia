import { dev } from '$app/environment';
import type { AppError } from '$types/bindings';

export function formatAppError(error: AppError | string | unknown): string {
    if (!isAppError(error)) return String(error);

    const message = appErrorMessage(error);
    if (!dev) return message;

    const details = appErrorDetails(error);
    return details ? `${message}\n${details}` : message;
}

export function appErrorMessage(error: AppError): string {
    switch (error.kind) {
        case 'core':
        case 'convert':
        case 'source':
            return error.message;
        case 'io':
            return `File system error: ${error.message}`;
        case 'json':
            return `Settings data could not be read: ${error.message}`;
        case 'semaphore':
        case 'task':
            return `Background task failed: ${error.message}`;
        case 'open_path':
            return `Could not open the folder: ${error.message}`;
        case 'state_lock':
            return `Internal state is temporarily unavailable: ${error.message}`;
        case 'suwayomi_not_installed':
            return 'Suwayomi-Server is not installed.';
        case 'suwayomi_not_running':
            return 'Suwayomi-Server is not running.';
        case 'suwayomi_starting':
            return 'Suwayomi-Server is still starting.';
        case 'suwayomi_runtime':
            return `Suwayomi-Server is not running: ${error.message}`;
        case 'suwayomi_not_ready':
            return 'Suwayomi-Server is not ready yet.';
        case 'cancelled':
            return 'Cancelled.';
        case 'message':
            return error.message;
    }
}

function appErrorDetails(error: AppError): string {
    const parts = [`kind: ${error.kind}`];
    if ('code' in error) parts.push(`code: ${error.code}`);
    if ('severity' in error) parts.push(`severity: ${error.severity}`);
    if ('causes' in error && error.causes.length > 0) {
        parts.push(`causes: ${error.causes.join(' -> ')}`);
    }
    return parts.join('\n');
}

function isAppError(error: unknown): error is AppError {
    return typeof error === 'object' && error !== null && 'kind' in error;
}
