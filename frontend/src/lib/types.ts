import type { components, operations } from '$lib/api/schema';

export type MetricsResponse =
    operations['getCheckMetrics']['responses']['200']['content']['application/json'];

export type CheckWithMetrics = components['schemas']['CheckWithAccess'] & {
    metrics?: MetricsResponse;
};

export type Region = components['schemas']['Region'];
