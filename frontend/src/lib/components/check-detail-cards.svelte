<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { REGION_LABELS } from '$lib/constants';
	import type { components } from '$lib/api/schema';

	type Check = components['schemas']['CheckWithAccess'];

	interface Props {
		check: Check;
	}

	let { check }: Props = $props();

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleString();
	}
</script>

<div class="grid gap-6 md:grid-cols-2">
	<Card.Root>
		<Card.Header>
			<Card.Title>General Information</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-4">
			<div>
				<div class="text-sm font-medium text-muted-foreground">Status</div>
				<div class="mt-1">
					{#if check.is_enabled}
						<Badge variant="default">Enabled</Badge>
					{:else}
						<Badge variant="secondary">Disabled</Badge>
					{/if}
				</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">URL</div>
				<div class="mt-1 text-sm break-all">{check.url}</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">HTTP Method</div>
				<div class="mt-1">
					<Badge variant="outline">{check.http_method}</Badge>
				</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">Expected Status Code</div>
				<div class="mt-1 text-sm">{check.expected_status_code}</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">Created At</div>
				<div class="mt-1 text-sm">{formatDate(check.created_at)}</div>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Monitoring Configuration</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-4">
			<div>
				<div class="text-sm font-medium text-muted-foreground">Check Frequency</div>
				<div class="mt-1 text-sm">{check.check_frequency_seconds} seconds</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">Timeout</div>
				<div class="mt-1 text-sm">{check.timeout_seconds} seconds</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">Regions</div>
				<div class="mt-2 flex flex-wrap gap-2">
					{#each check.regions as region (region)}
						<Badge variant="secondary">{REGION_LABELS[region]}</Badge>
					{/each}
				</div>
			</div>
			<div>
				<div class="text-sm font-medium text-muted-foreground">Permissions</div>
				<div class="mt-2 flex gap-2">
					{#if check.can_see}
						<Badge variant="outline">Can View</Badge>
					{/if}
					{#if check.can_edit}
						<Badge variant="outline">Can Edit</Badge>
					{/if}
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>

{#if Object.keys(check.request_headers).length > 0 || check.request_body}
	<Card.Root>
		<Card.Header>
			<Card.Title>Request Details</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-4">
			{#if Object.keys(check.request_headers).length > 0}
				<div>
					<div class="text-sm font-medium text-muted-foreground">Request Headers</div>
					<div class="mt-2 rounded-md bg-muted p-3 font-mono text-xs">
						{#each Object.entries(check.request_headers) as [key, value] (key)}
							<div>{key}: {value}</div>
						{/each}
					</div>
				</div>
			{/if}
			{#if check.request_body}
				<div>
					<div class="text-sm font-medium text-muted-foreground">Request Body</div>
					<div class="mt-2 rounded-md bg-muted p-3 font-mono text-xs break-all">
						{check.request_body}
					</div>
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
{/if}
