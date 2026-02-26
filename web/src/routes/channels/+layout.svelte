<script lang="ts">
	import { auth } from '$lib/stores/auth.svelte';
	import { messages } from '$lib/stores/messages.svelte';
	import { api, type GuildResponse, type ChannelResponse } from '$lib/services/api';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { voice } from '$lib/services/voice.svelte';
	import VoicePanel from '$lib/components/VoicePanel.svelte';

	let { children } = $props();

	let guilds: GuildResponse[] = $state([]);
	let selectedGuild: GuildResponse | null = $state(null);
	let channels: ChannelResponse[] = $state([]);
	let showCreateGuild = $state(false);
	let newGuildName = $state('');

	let inviteCode = $state('');
	let showInvite = $state(false);

	let showCreateChannel = $state(false);
	let newChannelName = $state('');
	let newChannelKind = $state<'text' | 'voice'>('text');

	onMount(async () => {
		if (!auth.user) {
			goto('/auth');
			return;
		}
		messages.init();
		await loadGuilds();
	});

	async function loadGuilds() {
		try {
			guilds = await api.listGuilds();
			if (guilds.length > 0 && !selectedGuild) {
				await selectGuild(guilds[0]);
			}
		} catch (e) {
			console.error('Failed to load guilds', e);
		}
	}

	async function selectGuild(guild: GuildResponse) {
		selectedGuild = guild;
		try {
			channels = await api.listChannels(guild.id);
			// Naviguer vers le premier channel text du guild
			const firstText = channels.find((c) => c.kind === 'text');
			if (firstText) {
				goto(`/channels/${firstText.id}`);
			} else {
				goto('/channels');
			}
		} catch (e) {
			console.error('Failed to load channels', e);
		}
	}

	async function handleCreateGuild() {
		if (!newGuildName.trim()) return;
		try {
			const guild = await api.createGuild(newGuildName.trim());
			newGuildName = '';
			showCreateGuild = false;
			await loadGuilds();
			await selectGuild(guild);
		} catch (e) {
			console.error('Failed to create guild', e);
		}
	}

	async function handleCreateInvite() {
		if (!selectedGuild) return;
		try {
			const invite = await api.createInvite(selectedGuild.id);
			inviteCode = `${window.location.origin}/invite/${invite.code}`;
			showInvite = true;
		} catch (e) {
			console.error('Failed to create invite', e);
		}
	}

	async function handleCreateChannel() {
		if (!selectedGuild || !newChannelName.trim()) return;
		try {
			await api.createChannel(selectedGuild.id, newChannelName.trim(), newChannelKind);
			newChannelName = '';
			showCreateChannel = false;
			// Recharger les channels
			channels = await api.listChannels(selectedGuild.id);
		} catch (e) {
			console.error('Failed to create channel', e);
		}
	}

	function copyInvite() {
		navigator.clipboard.writeText(inviteCode);
	}
</script>

<div class="flex h-full">
	<!-- Guild sidebar -->
	<aside class="flex w-16 flex-col items-center gap-2 bg-zinc-950 py-3">
		{#each guilds as guild (guild.id)}
			<button
				class="flex h-12 w-12 items-center justify-center rounded-2xl text-sm font-bold transition-all hover:rounded-xl {selectedGuild?.id ===
				guild.id
					? 'rounded-xl bg-indigo-600'
					: 'bg-zinc-700 hover:bg-zinc-600'}"
				onclick={() => selectGuild(guild)}
				title={guild.name}
			>
				{guild.name.charAt(0).toUpperCase()}
			</button>
		{/each}

		<Separator class="mx-auto w-8" />

		<button
			class="flex h-12 w-12 items-center justify-center rounded-2xl bg-zinc-800 text-xl text-green-500 transition-all hover:rounded-xl hover:bg-green-600 hover:text-white"
			onclick={() => (showCreateGuild = !showCreateGuild)}
			title="Create a server"
		>
			+
		</button>
	</aside>

	<!-- Channel sidebar -->
	<aside class="flex w-56 flex-col bg-zinc-900">
		<div class="flex h-12 items-center justify-between px-4 font-semibold shadow-md">
			{selectedGuild?.name ?? 'Pulsar'}
			{#if selectedGuild}
				<button
					class="text-xs text-muted-foreground hover:text-zinc-100"
					onclick={handleCreateInvite}
					title="Create invite"
				>
					+Invite
				</button>
			{/if}
		</div>
		<Separator />

		{#if showInvite}
			<div class="space-y-2 p-3">
				<p class="text-xs text-muted-foreground">Share this link:</p>
				<div class="flex gap-1">
					<input
						readonly
						value={inviteCode}
						class="flex-1 rounded bg-zinc-800 px-2 py-1.5 text-xs outline-none"
					/>
					<Button size="sm" variant="outline" onclick={copyInvite}>Copy</Button>
				</div>
				<button
					class="text-xs text-muted-foreground hover:text-zinc-100"
					onclick={() => (showInvite = false)}
				>
					Dismiss
				</button>
			</div>
			<Separator />
		{/if}

		{#if showCreateGuild}
			<div class="space-y-2 p-3">
				<input
					bind:value={newGuildName}
					placeholder="Server name"
					class="w-full rounded bg-zinc-800 px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
					onkeydown={(e) => e.key === 'Enter' && handleCreateGuild()}
				/>
				<Button class="w-full" size="sm" onclick={handleCreateGuild}>Create Server</Button>
			</div>
			<Separator />
		{/if}

		{#if selectedGuild}
			<div class="flex items-center justify-between px-3 pt-2">
				<span class="text-xs text-muted-foreground">Channels</span>
				<button
					class="text-xs text-muted-foreground hover:text-zinc-100"
					onclick={() => (showCreateChannel = !showCreateChannel)}
				>
					+
				</button>
			</div>

			{#if showCreateChannel}
				<div class="space-y-2 p-3">
					<input
						bind:value={newChannelName}
						placeholder="Channel name"
						class="w-full rounded bg-zinc-800 px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
						onkeydown={(e) => e.key === 'Enter' && handleCreateChannel()}
					/>
					<div class="flex gap-2">
						<button
							class="flex-1 rounded px-2 py-1.5 text-xs {newChannelKind === 'text' ? 'bg-indigo-600' : 'bg-zinc-800'}"
							onclick={() => (newChannelKind = 'text')}
						>
							# Text
						</button>
						<button
							class="flex-1 rounded px-2 py-1.5 text-xs {newChannelKind === 'voice' ? 'bg-indigo-600' : 'bg-zinc-800'}"
							onclick={() => (newChannelKind = 'voice')}
						>
							🔊 Voice
						</button>
					</div>
					<Button class="w-full" size="sm" onclick={handleCreateChannel}>
						Create
					</Button>
				</div>
			{/if}
		{/if}

		<nav class="flex-1 space-y-0.5 overflow-y-auto p-2">
			{#if channels.length > 0}
				<p class="px-2 py-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
					Text Channels
				</p>
				{#each channels.filter((c) => c.kind === 'text') as channel (channel.id)}
					<a
						href="/channels/{channel.id}"
						class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100"
					>
						<span class="text-lg text-muted-foreground">#</span>
						{channel.name}
					</a>
				{/each}

				{#if channels.filter((c) => c.kind === 'voice').length > 0}
					<p
						class="mt-3 px-2 py-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase"
					>
						Voice Channels
					</p>
					{#each channels.filter((c) => c.kind === 'voice') as channel (channel.id)}
						<button
							class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-100
						{voice.currentChannelId === channel.id ? 'bg-zinc-800 text-zinc-100' : ''}"
							onclick={() => voice.join(channel.id)}
						>
							<span class="text-lg text-muted-foreground">🔊</span>
							{channel.name}
							{#if voice.currentChannelId === channel.id}
								<span class="ml-auto text-xs text-green-500">●</span>
							{/if}
						</button>
					{/each}
				{/if}
			{:else if selectedGuild}
				<p class="px-2 py-4 text-center text-sm text-muted-foreground">No channels yet</p>
			{:else}
				<p class="px-2 py-4 text-center text-sm text-muted-foreground">Create or join a server</p>
			{/if}
		</nav>

		<VoicePanel />

		<Separator />

		<div class="flex items-center gap-2 p-3">
			<div
				class="flex h-8 w-8 items-center justify-center rounded-full bg-indigo-600 text-sm font-medium"
			>
				{auth.user?.username.charAt(0).toUpperCase()}
			</div>
			<div class="flex-1 truncate text-sm font-medium">
				{auth.user?.username}
			</div>
			<button
				class="text-xs text-muted-foreground hover:text-zinc-100"
				onclick={() => {
					auth.logout();
					goto('/auth');
				}}
			>
				Logout
			</button>
		</div>
	</aside>

	<!-- Main content -->
	<main class="flex flex-1 flex-col bg-zinc-950">
		{@render children()}
	</main>
</div>
