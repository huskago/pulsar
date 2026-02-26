<script lang="ts">
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import { api, type InviteResponse } from '$lib/services/api';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';

	let code = $derived((page.params as { code: string }).code ?? '');
	let invite: InviteResponse | null = $state(null);
	let error = $state('');
	let loading = $state(true);
	let joining = $state(false);

	onMount(async () => {
		try {
			invite = await api.getInvite(code);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Invite not found';
		} finally {
			loading = false;
		}
	});

	async function handleJoin() {
		if (!auth.user) {
			// Pas connecté → redirect vers auth avec retour
			localStorage.setItem('pulsar_pending_invite', code);
			goto('/auth');
			return;
		}

		joining = true;
		try {
			await api.joinInvite(code);
			goto('/channels');
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to join';
		} finally {
			joining = false;
		}
	}
</script>

<div class="flex h-screen items-center justify-center bg-zinc-950">
	{#if loading}
		<p class="text-muted-foreground">Loading invite...</p>
	{:else if error}
		<Card.Root class="w-full max-w-sm p-8 text-center">
			<Card.Header>
				<Card.Title class="text-xl">Invalid Invite</Card.Title>
				<Card.Description>{error}</Card.Description>
			</Card.Header>
			<Card.Footer class="justify-center">
				<Button variant="outline" onclick={() => goto('/')}>Go Home</Button>
			</Card.Footer>
		</Card.Root>
	{:else if invite}
		<Card.Root class="w-full max-w-sm p-8 text-center">
			<Card.Header>
				<div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-indigo-600 text-2xl font-bold">
					{invite.guild_name.charAt(0).toUpperCase()}
				</div>
				<Card.Description class="text-xs uppercase tracking-wide">
					You've been invited to join
				</Card.Description>
				<Card.Title class="text-2xl">{invite.guild_name}</Card.Title>
			</Card.Header>
			<Card.Footer class="justify-center">
				<Button class="w-full" onclick={handleJoin} disabled={joining}>
					{joining ? 'Joining...' : auth.user ? 'Accept Invite' : 'Login to Join'}
				</Button>
			</Card.Footer>
		</Card.Root>
	{/if}
</div>