<script lang="ts">
    interface Props {
        uptime: number;
    }

    let { uptime }: Props = $props();

    function getUptimeColor(value: number): string {
        if (value >= 99) return 'bg-green-500';
        if (value >= 95) return 'bg-yellow-500';
        return 'bg-red-500';
    }
</script>

<div class="flex items-center gap-2">
    <span class="h-2 w-2 rounded-full {getUptimeColor(uptime)}"></span>
    <span class="text-sm font-medium text-muted-foreground">
        {uptime.toFixed(1)}%
    </span>
</div>
