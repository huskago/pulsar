import { SvelteSet } from 'svelte/reactivity';
import type { Message } from '$lib/types/models';
import type { ServerEvent } from '$lib/types/events';
import { gateway } from '$lib/services/gateway.svelte';

let messagesByChannel: Record<string, Message[]> = $state({});
let typingByChannel: Record<string, SvelteSet<string>> = $state({});

function init() {
	gateway.onEvent((event: ServerEvent) => {
		switch (event.type) {
			case 'MessageCreate': {
				const msg = event.data;
				const channelId = msg.channel_id;

				if (!messagesByChannel[channelId]) {
					messagesByChannel[channelId] = [];
				}

				const exists = messagesByChannel[channelId].some((m) => m.id === msg.id);
				if (!exists) {
					messagesByChannel[channelId] = [...messagesByChannel[channelId], msg];
				}
				break;
			}

			case 'TypingStart': {
				const { channel_id, user_id } = event.data;
				if (!typingByChannel[channel_id]) {
					typingByChannel[channel_id] = new SvelteSet();
				}
				typingByChannel[channel_id].add(user_id);

				setTimeout(() => {
					typingByChannel[channel_id]?.delete(user_id);
				}, 5000);
				break;
			}
		}
	});
}

function getMessages(channelId: string): Message[] {
	return messagesByChannel[channelId] ?? [];
}

function getTypingUsers(channelId: string): string[] {
	return typingByChannel[channelId] ? [...typingByChannel[channelId]] : [];
}

function sendMessage(channelId: string, content: string) {
	gateway.sendMessage(channelId, content);
}

export const messages = {
	init,
	getMessages,
	getTypingUsers,
	sendMessage
};
