import createClient from 'openapi-fetch';
import type { paths } from './schema';
import { env } from '$env/dynamic/public';

// Create client with public backend URL
// Server-side requests are automatically rewritten to private IPs via handleFetch hook
// This ensures cache keys are consistent between SSR and client hydration
export const api = createClient<paths>({
    baseUrl: env.PUBLIC_BACKEND_URL,
    credentials: 'include'
});
