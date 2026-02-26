<script lang="ts">
	import { page } from '$app/state';
	import { messages } from '$lib/stores/messages.svelte';
	import { gateway } from '$lib/services/gateway.svelte';
	import { api, type MessageResponse, type UploadResponse } from '$lib/services/api';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { tick } from 'svelte';
	import type { Message } from '$lib/types/models';
	import VideoGrid from '$lib/components/VideoGrid.svelte';
	import { voice } from '$lib/services/voice.svelte';

	let channelId = $derived(page.params.id ?? '');
	let channelMessages = $derived(messages.getMessages(channelId));
	let typingUsers = $derived(messages.getTypingUsers(channelId));

	let inputValue = $state('');
	let messagesContainer: HTMLDivElement | undefined = $state();
	let loadingHistory = $state(false);

	let fileInput: HTMLInputElement;
	let uploading = $state(false);
	let pendingAttachments: UploadResponse[] = $state([]);

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
						attachments: m.attachments ?? [],
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
		if (!content && pendingAttachments.length === 0) return;
		messages.sendMessage(
			channelId,
			content,
			pendingAttachments.map((a) => ({
				filename: a.filename,
				content_type: a.content_type,
				size: a.size,
				url: a.url
			}))
		);
		inputValue = '';
		pendingAttachments = [];
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

	async function handleFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		if (!input.files?.length) return;

		uploading = true;
		try {
			for (const file of Array.from(input.files)) {
				const result = await api.uploadFile(channelId, file);
				pendingAttachments = [...pendingAttachments, result];
			}
		} catch (e) {
			console.error('Upload failed:', e);
		} finally {
			uploading = false;
			input.value = '';
		}
	}

	function removePending(index: number) {
		pendingAttachments = pendingAttachments.filter((_, i) => i !== index);
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

{#if voice.connected && voice.participants.some((p) => p.videoTracks.length > 0)}
	<div class="max-h-[50vh] overflow-y-auto border-b border-zinc-800">
		<VideoGrid />
	</div>
{/if}

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
		{#each channelMessages as  msg (msg.id)}
			<div class="group flex gap-3 rounded px-2 py-1 hover:bg-zinc-900/50">
				<div
					class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-indigo-600 text-xs font-medium"
				>
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
					<p class="text-sm wrap-break-word text-zinc-100">{msg.content}</p>

					<!-- Attachments -->
					{#if msg.attachments?.length}
						<div class="mt-2 space-y-2">
							{#each msg.attachments as att}
								{#if att.content_type.startsWith('image/')}
									<img
										src={att.url}
										alt={att.filename}
										class="max-h-80 max-w-md rounded-lg"
										loading="lazy"
									/>
								{:else if att.content_type.startsWith('video/')}
									<video
										src={att.url}
										controls
										class="max-h-80 max-w-md rounded-lg"
										preload="metadata"
									/>
								{:else if att.content_type.startsWith('audio/')}
									<audio src={att.url} controls class="max-w-md" preload="metadata" />
								{:else}
									<a
										href={att.url}
										target="_blank"
										rel="noopener"
										class="inline-flex items-center gap-2 rounded bg-zinc-800 px-3 py-2 text-sm text-indigo-400 transition-colors hover:bg-zinc-700"
									>
										📎 {att.filename}
										<span class="text-xs text-muted-foreground">
											({(att.size / 1024).toFixed(0)} KB)
										</span>
									</a>
								{/if}
							{/each}
						</div>
					{/if}
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

<!-- Input area -->
<div class="border-t border-zinc-800 p-4">
	<!-- Pending attachments preview -->
	{#if pendingAttachments.length > 0}
		<div class="mb-2 flex flex-wrap gap-2">
			{#each pendingAttachments as att, i}
				<div class="relative rounded bg-zinc-800 p-2">
					{#if att.content_type.startsWith('image/')}
						<img src={att.url} alt={att.filename} class="h-20 w-20 rounded object-cover" />
					{:else}
						<div
							class="flex h-20 w-20 items-center justify-center rounded bg-zinc-700 text-xs text-zinc-400"
						>
							📎 {att.filename.split('.').pop()}
						</div>
					{/if}
					<button
						class="absolute -top-1 -right-1 flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-xs text-white"
						onclick={() => removePending(i)}
					>
						×
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<div class="flex gap-2">
		<!-- Upload button -->
		<button
			class="flex items-center justify-center rounded-lg bg-zinc-800 px-3 text-zinc-400 transition-colors hover:bg-zinc-700 hover:text-zinc-100"
			onclick={() => fileInput.click()}
			disabled={uploading}
			title="Upload file"
		>
			{uploading ? '⏳' : '📎'}
		</button>
		<input
			bind:this={fileInput}
			type="file"
			multiple
			accept="image/*,video/*,audio/*,.pdf,.txt,.zip,.rar,.7z,.doc,.docx,.xls,.xlsx"
			class="hidden"
			onchange={handleFileSelect}
		/>

		<!-- Message input -->
		<input
			bind:value={inputValue}
			placeholder="Message #{channelId}"
			class="flex-1 rounded-lg bg-zinc-800 px-4 py-3 text-sm outline-none focus:ring-1 focus:ring-indigo-500"
			onkeydown={(e) => {
				if (e.key === 'Enter' && !e.shiftKey) {
					e.preventDefault();
					handleSend();
				}
				handleKeydown(e);
			}}
		/>
	</div>
</div>