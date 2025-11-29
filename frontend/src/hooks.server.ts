import type { Handle, HandleFetch } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';

export const handle: Handle = async ({ event, resolve }) => {
    return await resolve(event, {
        filterSerializedResponseHeaders: (name) => {
            return name === 'content-length';
        }
    });
};

// Rewrite public API URLs to private IPs during SSR for better performance
// while keeping cache keys consistent for client hydration
export const handleFetch: HandleFetch = async ({ event, request, fetch }) => {
    if (env.PUBLIC_BACKEND_URL && env.PUBLIC_PRIVATE_API_URL) {
        if (request.url.startsWith(env.PUBLIC_BACKEND_URL)) {
            // Clone the request with the private API URL for faster server-side access
            request = new Request(
                request.url.replace(env.PUBLIC_BACKEND_URL, env.PUBLIC_PRIVATE_API_URL),
                request
            );

            // Forward cookies from the original browser request to the backend
            // This is necessary because we're rewriting to a different origin (private IP)
            const cookieHeader = event.request.headers.get('cookie');
            if (cookieHeader) {
                request.headers.set('cookie', cookieHeader);
            }
        }
    }

    return fetch(request);
};
