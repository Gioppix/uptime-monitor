import { api } from '$lib/api/client';
import type { CheckWithMetrics } from '$lib/types';
import { getMinuteDateRange24Hours } from '$lib/utils';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
    const checksResult = await api.GET('/checks/', { fetch });
    const checks = checksResult.data || [];

    const { from, to } = getMinuteDateRange24Hours();

    const checksWithMetrics: CheckWithMetrics[] = await Promise.all(
        checks.map(async (check) => {
            if (!check.is_enabled) return check;

            const metricsResult = await api.GET('/checks/{check_id}/metrics', {
                params: { path: { check_id: check.check_id }, query: { from, to } },
                fetch
            });

            return { ...check, metrics: metricsResult.data };
        })
    );

    return { checks: checksWithMetrics };
};
