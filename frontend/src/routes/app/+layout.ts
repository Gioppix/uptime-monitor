import { api } from '$lib/api/client';
import type { LayoutLoad } from './$types';

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
