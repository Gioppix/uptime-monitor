import { api } from '$lib/api/client';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
    const result = await api.GET('/checks/', { fetch });

    return {
        checks: result.data || []
    };
};
