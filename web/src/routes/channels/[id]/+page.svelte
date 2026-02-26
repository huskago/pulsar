<script lang="ts">
	import { page } from '$app/state';
	import { messages } from '$lib/stores/messages.svelte';
	import { gateway } from '$lib/services/gateway.svelte';
	import { api, type MessageResponse } from '$lib/services/api';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { tick, onMount } from 'svelte';
	import type { Message } from '$lib/types/models';

	let channelId = $derived(page.params.id ?? '');
	let channelMessages = $derived(messages.getMessages(channelId));
	let typingUsers = $derived(messages.getTypingUsers(channelId));

	let inputValue = $state('');
	let messagesContainer: HTMLDivElement | undefined = $state();
	let loadingHistory = $state(false);

	let typingTimeout: ReturnType<typeof setTimeout> | null = null;

	// Charger l'historique quand on change de channel
	$effect(() => {
		const id = channelId;
		if (id) {
			loadHistory(id);
		}
	});

	async function loadHistory(chId: string) {
		loadingHistory = true;
		try {
			const history: MessageResponse[] = await api.listMessages(chId, 50);
			// Injecter dans le store
			messages.setMessages(
				chId,
				history.map(
					(m): Message => ({
						id: m.id,
						channel_id: m.channel_id,
						author_id: m.author_id,
						content: m.content,
						timestamp: m.timestamp,
						edited_timestamp: m.edited_timestamp
					})
				)
			);
		} catch (e) {
			console.error('Failed to load history', e);
		} finally {
			loadingHistory = false;
		}
	}

	$effect(() => {
		void channelMessages.length;
		tick().then(() => {
			if (messagesContainer) {
				messagesContainer.scrollTop = messagesContainer.scrollHeight;
			}
		});
	});

	function handleSend() {
		const content = inputValue.trim();
		if (!content) return;
		messages.sendMessage(channelId, content);
		inputValue = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSend();
			return;
		}

		// Envoyer typing indicator (debounced à 3s)
		if (!typingTimeout) {
			gateway.startTyping(channelId);
			typingTimeout = setTimeout(() => {
				typingTimeout = null;
			}, 3000);
		}
	}

	function formatTime(timestamp: number): string {
		return new Date(timestamp).toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div class="flex h-12 items-center gap-2 px-4 shadow-md">
	<span class="text-lg text-muted-foreground">#</span>
	<span class="font-semibold">Channel</span>
	{#if gateway.connected}
		<span class="ml-auto text-xs text-green-500">● Connected</span>
	{:else if gateway.reconnecting}
		<span class="ml-auto text-xs text-yellow-500">● Reconnecting...</span>
	{:else}
		<span class="ml-auto text-xs text-red-500">● Disconnected</span>
	{/if}
</div>

<Separator />

<div bind:this={messagesContainer} class="flex-1 space-y-1 overflow-y-auto p-4">
	{#if loadingHistory}
		<div class="flex h-full items-center justify-center text-muted-foreground">
			<p>Loading messages...</p>
		</div>
	{:else if channelMessages.length === 0}
		<div class="flex h-full items-center justify-center text-muted-foreground">
			<p>No messages yet. Say something!</p>
		</div>
	{:else}
		{#each channelMessages as msg (msg.id)}
			<div class="group flex gap-3 rounded px-2 py-1 hover:bg-zinc-900/50">
				<div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-indigo-600 text-xs font-medium">
					{String(msg.author_id).slice(-2)}
				</div>
				<div class="min-w-0 flex-1">
					<div class="flex items-baseline gap-2">
						<span class="text-sm font-semibold text-zinc-200">
							User {String(msg.author_id).slice(-4)}
						</span>
						<span class="text-xs text-muted-foreground">
							{formatTime(msg.timestamp)}
						</span>
					</div>
					<p class="wrap-break-word text-sm text-zinc-300">{msg.content}</p>
				</div>
			</div>
		{/each}
	{/if}
</div>

{#if typingUsers.length > 0}
	<div class="px-4 py-1 text-xs text-muted-foreground">
		{typingUsers.length === 1
			? `User ${typingUsers[0].slice(-4)} is typing...`
			: `${typingUsers.length} people are typing...`}
	</div>
{/if}

<div class="p-4 pt-0">
	<div class="flex gap-2">
		<Input
			bind:value={inputValue}
			placeholder="Message this channel"
			onkeydown={handleKeydown}
			class="flex-1 border-zinc-700 bg-zinc-900"
		/>
		<Button onclick={handleSend} disabled={!inputValue.trim()}>Send</Button>
	</div>
</div>