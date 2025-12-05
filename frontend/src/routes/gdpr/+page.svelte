<script lang="ts">
    import LandingHeader from '$lib/components/landing-header.svelte';

    const { data } = $props();
</script>

<div class="min-h-screen">
    <LandingHeader user={data.user} showBackButton />

    <main class="mx-auto max-w-2xl px-6 py-16">
        <h1 class="text-3xl font-bold">GDPR Compliance</h1>

        <img
            src="https://media.giphy.com/media/JIX9t2j0ZTN9S/giphy.gif"
            alt="Cat typing"
            class="my-8 w-full rounded-lg"
        />

        <div class="space-y-6 text-muted-foreground">
            <p>We take your privacy very seriously.</p>

            <h2 class="text-xl font-semibold text-foreground">Data We Collect</h2>
            <p>
                Everything. Your email, your endpoints, your IP address, your browser fingerprint,
                your mouse movements, and that thing you searched for at 2am (it required SPID).
            </p>

            <h2 class="text-xl font-semibold text-foreground">Data Deletion</h2>
            <p>
                When you delete your account, we perform a soft delete. This means we set
                <span class="font-mono">deleted_at = NOW()</span> and pretend the data doesn't exist,
                just like the rest of the internet. Your data lives forever in our backups, archived
                logs, and that one developer's local database dump from 2019.
            </p>

            <h2 class="text-xl font-semibold text-foreground">Your Rights</h2>
            <p>
                You have the right to request your data. We have the right to take 30 business days
                to respond. You have the right to be forgotten. We have the right to forget that you
                asked.
            </p>

            <h2 class="text-xl font-semibold text-foreground">Cookies</h2>
            <p>
                By continuing to exist on this website, you consent to all cookies, future cookies,
                and cookies that haven't been invented yet. We also share data with 463 "trusted
                partners" whose names we lost in a spreadsheet somewhere.
            </p>

            <h2 class="text-xl font-semibold text-foreground">Third Parties</h2>
            <p>
                We share your data with third parties for "legitimate business purposes," which is
                legal speak for "money." These third parties have pinky-promised to be nice with
                your data.
            </p>

            <h2 class="text-xl font-semibold text-foreground">Contact</h2>
            <p>
                Questions? Complaints? Email our DPO at
                <span class="font-mono">/dev/null</span>. Average response time is 6-8 weeks,
                depending on how good my Instagram feed is.
            </p>
        </div>
    </main>
</div>
