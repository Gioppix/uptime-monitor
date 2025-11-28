import createClient from 'openapi-fetch';
import type { paths } from './schema';
import { env } from '$env/dynamic/public';

// Create client with getter for baseUrl to ensure env is read at request time
export const api = createClient<paths>({
    get baseUrl() {
        // Server-side: use private API URL from environment
        if (typeof window === 'undefined') {
            return env.PUBLIC_PRIVATE_API_URL;
        }
        // Client-side: use current hostname with backend port
        return `http://${window.location.hostname}:${env.PUBLIC_BACKEND_PORT}`;
    },
    credentials: 'include'
});
