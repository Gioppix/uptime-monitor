import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, 'child'> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, 'children'> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };

export function getUserInitial(username: string): string {
    return username.charAt(0).toUpperCase();
}

export function getUserColor(userId: string): string {
    let hash = 0;
    for (let i = 0; i < userId.length; i++) {
        hash = userId.charCodeAt(i) + ((hash << 5) - hash);
    }
    const hue = Math.abs(hash) % 360;
    return `hsl(${hue}, 70%, 60%)`;
}

export function getMinuteDateRange24Hours() {
    const now = new Date();
    now.setSeconds(0, 0);

    const endOfCurrentMinute = now.getTime() + 60 * 1000;
    const twentyFourHoursAgo = endOfCurrentMinute - 24 * 60 * 60 * 1000;

    return {
        from: new Date(twentyFourHoursAgo).toISOString(),
        to: new Date(endOfCurrentMinute).toISOString()
    };
}
