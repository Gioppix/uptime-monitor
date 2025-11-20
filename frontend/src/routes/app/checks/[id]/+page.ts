import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
    const result = await api.GET('/checks/{check_id}', {
        params: { path: { check_id: params.id } },
        fetch
    });

    if (result.error || !result.data) {
        if (result.response?.status === 404) {
            error(404, 'Check not found');
        }
        error(500, 'Failed to load check');
    }

    return {
        check: result.data
    };
};
