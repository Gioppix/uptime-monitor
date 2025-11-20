<script lang="ts">
    import { api } from '$lib/api/client';
    import { invalidateAll } from '$app/navigation';
    import { Button } from '$lib/components/ui/button';
    import * as Dialog from '$lib/components/ui/dialog';
    import { Input } from '$lib/components/ui/input';
    import { Label } from '$lib/components/ui/label';
    import { ALL_METHODS, ALL_REGIONS, REGION_LABELS } from '$lib/constants';
    import type { components } from '$lib/api/schema';
    import type { Method, Region } from '$lib/constants';

    type Check = components['schemas']['CheckWithAccess'];

    interface Props {
        open: boolean;
        check?: Check | null;
        onOpenChange?: (open: boolean) => void;
    }

    let { open = $bindable(), check = null, onOpenChange }: Props = $props();

    let isSubmitting = $state(false);

    let formData = $derived.by(() => {
        if (check) {
            return {
                check_name: check.check_name,
                url: check.url,
                http_method: check.http_method,
                expected_status_code: check.expected_status_code,
                check_frequency_seconds: check.check_frequency_seconds,
                timeout_seconds: check.timeout_seconds,
                is_enabled: check.is_enabled,
                request_body: check.request_body || '',
                regions: check.regions
            };
        }
        return {
            check_name: '',
            url: '',
            http_method: 'GET' as Method,
            expected_status_code: 200,
            check_frequency_seconds: 60,
            timeout_seconds: 30,
            is_enabled: true,
            request_body: '',
            regions: ['UsWest'] as Region[]
        };
    });

    let headersText = $derived(
        check
            ? Object.entries(check.request_headers)
                  .map(([k, v]) => `${k}: ${v}`)
                  .join('\n')
            : ''
    );

    function parseHeaders(text: string): Record<string, string> {
        const headers: Record<string, string> = {};
        text.split('\n').forEach((line) => {
            const colonIndex = line.indexOf(':');
            if (colonIndex > 0) {
                const key = line.substring(0, colonIndex).trim();
                const value = line.substring(colonIndex + 1).trim();
                if (key) headers[key] = value;
            }
        });
        return headers;
    }

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        if (isSubmitting) return;
        isSubmitting = true;

        try {
            const form = event.target as HTMLFormElement;
            const formDataObj = new FormData(form);

            const regions: Region[] = [];
            ALL_REGIONS.forEach((region) => {
                if (formDataObj.get(region) === 'on') {
                    regions.push(region);
                }
            });

            const checkData = {
                check_name: formDataObj.get('check_name') as string,
                url: formDataObj.get('url') as string,
                http_method: formDataObj.get('http_method') as Method,
                expected_status_code: parseInt(formDataObj.get('expected_status_code') as string),
                check_frequency_seconds: parseInt(
                    formDataObj.get('check_frequency_seconds') as string
                ),
                timeout_seconds: parseInt(formDataObj.get('timeout_seconds') as string),
                is_enabled: formDataObj.get('is_enabled') === 'on',
                request_headers: parseHeaders(formDataObj.get('request_headers') as string),
                request_body: (formDataObj.get('request_body') as string) || null,
                regions
            };

            if (check) {
                await api.PATCH('/checks/{check_id}', {
                    params: { path: { check_id: check.check_id } },
                    body: {
                        check_id: check.check_id,
                        created_at: check.created_at,
                        ...checkData
                    }
                });
            } else {
                await api.POST('/checks/', {
                    body: {
                        check_id: crypto.randomUUID(),
                        created_at: new Date().toISOString(),
                        ...checkData
                    }
                });
            }

            open = false;
            await invalidateAll();
        } finally {
            isSubmitting = false;
        }
    }

    function handleOpenChange(newOpen: boolean) {
        open = newOpen;
        onOpenChange?.(newOpen);
    }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
    <Dialog.Content class="max-h-[90vh] max-w-2xl overflow-y-auto">
        <Dialog.Header>
            <Dialog.Title>{check ? 'Edit Check' : 'Create New Check'}</Dialog.Title>
            <Dialog.Description>
                {check
                    ? 'Update the check configuration'
                    : 'Configure a new uptime monitoring check'}
            </Dialog.Description>
        </Dialog.Header>

        <form onsubmit={handleSubmit} class="space-y-4">
            <div class="space-y-2">
                <Label for="check_name">Check Name</Label>
                <Input
                    id="check_name"
                    name="check_name"
                    value={formData.check_name}
                    placeholder="My API Check"
                    required
                />
            </div>

            <div class="space-y-2">
                <Label for="url">URL</Label>
                <Input
                    id="url"
                    name="url"
                    type="url"
                    value={formData.url}
                    placeholder="https://api.example.com/health"
                    required
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                    <Label for="http_method">HTTP Method</Label>
                    <select
                        id="http_method"
                        name="http_method"
                        value={formData.http_method}
                        class="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
                    >
                        {#each ALL_METHODS as method (method)}
                            <option value={method}>{method}</option>
                        {/each}
                    </select>
                </div>

                <div class="space-y-2">
                    <Label for="expected_status_code">Expected Status Code</Label>
                    <Input
                        id="expected_status_code"
                        name="expected_status_code"
                        type="number"
                        value={formData.expected_status_code}
                        min="100"
                        max="599"
                        required
                    />
                </div>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                    <Label for="check_frequency_seconds">Check Frequency (seconds)</Label>
                    <Input
                        id="check_frequency_seconds"
                        name="check_frequency_seconds"
                        type="number"
                        value={formData.check_frequency_seconds}
                        min="10"
                        required
                    />
                </div>

                <div class="space-y-2">
                    <Label for="timeout_seconds">Timeout (seconds)</Label>
                    <Input
                        id="timeout_seconds"
                        name="timeout_seconds"
                        type="number"
                        value={formData.timeout_seconds}
                        min="1"
                        required
                    />
                </div>
            </div>

            <div class="space-y-2">
                <Label>Regions</Label>
                <div class="grid grid-cols-2 gap-3">
                    {#each ALL_REGIONS as region (region)}
                        <div class="flex items-center space-x-2">
                            <input
                                type="checkbox"
                                name={region}
                                id={region}
                                checked={formData.regions.includes(region)}
                                class="h-4 w-4"
                            />
                            <label for={region} class="cursor-pointer text-sm">
                                {REGION_LABELS[region]}
                            </label>
                        </div>
                    {/each}
                </div>
            </div>

            <div class="space-y-2">
                <Label for="request_headers">Request Headers (optional)</Label>
                <textarea
                    id="request_headers"
                    name="request_headers"
                    value={headersText}
                    placeholder="Content-Type: application/json&#10;Authorization: Bearer token"
                    class="flex min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none"
                ></textarea>
            </div>

            <div class="space-y-2">
                <Label for="request_body">Request Body (optional)</Label>
                <textarea
                    id="request_body"
                    name="request_body"
                    value={formData.request_body}
                    placeholder="JSON request body"
                    class="flex min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none"
                ></textarea>
            </div>

            <div class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    id="is_enabled"
                    name="is_enabled"
                    checked={formData.is_enabled}
                    class="h-4 w-4"
                />
                <Label for="is_enabled">Enable check</Label>
            </div>

            <Dialog.Footer>
                <Button type="button" variant="outline" onclick={() => (open = false)}>
                    Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                    {isSubmitting ? 'Saving...' : check ? 'Update' : 'Create'}
                </Button>
            </Dialog.Footer>
        </form>
    </Dialog.Content>
</Dialog.Root>
