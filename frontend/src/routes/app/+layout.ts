import { api } from '$lib/api/client';
import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { resolve } from '$app/paths';

export const load: LayoutLoad = async ({ fetch }) => {
    const result = await api.GET('/users/me', {
        fetch
    });

    const user = result.data;

    // if (!user) {
    // 	redirect(307, resolve('/'));
    // }

    return {
        user
    };
};
