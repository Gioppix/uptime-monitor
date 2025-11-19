import { api } from '$lib/api/client';
import type { LayoutLoad } from './$types';

export const prerender = false;
export const ssr = true;

export const load: LayoutLoad = async ({ fetch }) => {
    const result = await api.GET('/users/me', {
        fetch
    });

    return {
        user: result.data
    };
};
