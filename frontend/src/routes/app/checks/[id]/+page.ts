import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getGraphTimes, getMinuteDateRange24Hours } from '$lib/utils';

export const load: PageLoad = async ({ params, fetch }) => {
    const { from, to } = getMinuteDateRange24Hours();

    const metricsPromise = api.GET('/checks/{check_id}/metrics', {
        params: { path: { check_id: params.id }, query: { from, to } },
        fetch
    });

    const { from: graphFrom, to: graphTo } = getGraphTimes({
        granularity: 'Hourly',
        days: 1
    });

    const defaultGraphPromise = api.GET('/checks/{check_id}/metrics/graph', {
        params: {
            path: { check_id: params.id },
            query: { from: graphFrom, to: graphTo, granularity: 'Hourly' }
        },
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

    if (!check.is_enabled) {
        return { check };
    }

    const [defaultGraph, metricsResult] = await Promise.all([defaultGraphPromise, metricsPromise]);

    return { check, metrics: metricsResult.data, defaultGraph: defaultGraph.data };
};
