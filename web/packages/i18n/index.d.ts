import type en from './locales/en.json';
export type MessageKey = keyof typeof en;
export declare const locales: { en: Record<string, string> };
export declare const defaultLocale: 'en';
export declare function t(key: MessageKey, params?: Record<string, string | number>): string;
