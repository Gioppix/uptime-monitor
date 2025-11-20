<script lang="ts">
    import * as Alert from '$lib/components/ui/alert';
    import Info from '@lucide/svelte/icons/info';
</script>

<Alert.Root>
    <Info class="h-4 w-4" />
    <Alert.Title>Not logged in</Alert.Title>
    <Alert.Description>
        Please log in to view and manage your uptime monitoring checks.
    </Alert.Description>
</Alert.Root>
