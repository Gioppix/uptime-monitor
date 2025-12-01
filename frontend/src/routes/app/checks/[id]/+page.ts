import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import type { MetricsResponse } from '$lib/types';

export const load: PageLoad = async ({ params, fetch }) => {
    const to = new Date().toISOString();
    const from = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString();

    const metricsPromise = api.GET('/checks/{check_id}/metrics', {
        params: { path: { check_id: params.id }, query: { from, to } },
        fetch
    });

    const checkResult = await api.GET('/checks/{check_id}', {
        params: { path: { check_id: params.id } },
        fetch
    });

    if (checkResult.error || !checkResult.data) {
        if (checkResult.response?.status === 404) {
            error(404, 'Check not found');
        }
        error(500, 'Failed to load check');
    }

    const check = checkResult.data;
    let metrics: MetricsResponse | undefined;

    if (check.is_enabled) {
        const metricsResult = await metricsPromise;
        metrics = metricsResult.data;
    }

    return { check, metrics };
};
