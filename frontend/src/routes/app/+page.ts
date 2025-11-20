import { api } from '$lib/api/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
    const result = await api.GET('/checks/', { fetch });

    const checks = result.data;

    return {
        checks
    };
};
