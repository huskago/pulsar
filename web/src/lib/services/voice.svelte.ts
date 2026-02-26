import {
	Room,
	RoomEvent,
	Track,
	type RemoteParticipant,
	type RemoteTrackPublication,
	type LocalParticipant
} from 'livekit-client';
import { api } from '$lib/services/api';

let room: Room | null = $state(null);
let connected = $state(false);
let connecting = $state(false);
let currentChannelId: string | null = $state(null);
let participants: ParticipantInfo[] = $state([]);
let isMuted = $state(false);

export interface ParticipantInfo {
	identity: string;
	name: string;
	isSpeaking: boolean;
	isMuted: boolean;
	isLocal: boolean;
}

function updateParticipants() {
	if (!room) {
		participants = [];
		return;
	}

	const list: ParticipantInfo[] = [];

	const local = room.localParticipant;
	list.push({
		identity: local.identity,
		name: local.name ?? local.identity,
		isSpeaking: local.isSpeaking,
		isMuted: !local.isMicrophoneEnabled,
		isLocal: true
	});

	room.remoteParticipants.forEach((p: RemoteParticipant) => {
		list.push({
			identity: p.identity,
			name: p.name ?? p.identity,
			isSpeaking: p.isSpeaking,
			isMuted: !p.isMicrophoneEnabled,
			isLocal: false
		});
	});

	participants = list;
}

async function join(channelId: string) {
	if (connected && currentChannelId === channelId) return;

	if (room) {
		await leave();
	}

	connecting = true;

	try {
		const res = await api.getVoiceToken(channelId);

		const newRoom = new Room({
			adaptiveStream: true,
			dynacast: true,
			videoCaptureDefaults: { resolution: { width: 0, height: 0, frameRate: 0 } }
		});

		newRoom.on(RoomEvent.ParticipantConnected, () => updateParticipants());
		newRoom.on(RoomEvent.ParticipantDisconnected, () => updateParticipants());
		newRoom.on(RoomEvent.ActiveSpeakersChanged, () => updateParticipants());
		newRoom.on(RoomEvent.TrackMuted, () => updateParticipants());
		newRoom.on(RoomEvent.TrackUnmuted, () => updateParticipants());
		newRoom.on(RoomEvent.LocalTrackPublished, () => updateParticipants());
		newRoom.on(RoomEvent.TrackSubscribed, () => updateParticipants());
		newRoom.on(RoomEvent.Disconnected, () => {
			connected = false;
			currentChannelId = null;
			participants = [];
		});

		newRoom.on(
			RoomEvent.TrackSubscribed,
			(track, _publication, _participant) => {
				if (track.kind === Track.Kind.Audio) {
					const el = track.attach();
					el.id = `lk-audio-${track.sid}`;
					document.body.appendChild(el);
				}
			}
		);

		newRoom.on(
			RoomEvent.TrackUnsubscribed,
			(track) => {
				track.detach().forEach((el) => el.remove());
			}
		);

		await newRoom.connect(res.url, res.token);
		await newRoom.localParticipant.setMicrophoneEnabled(true);

		room = newRoom;
		connected = true;
		connecting = false;
		currentChannelId = channelId;
		isMuted = false;

		updateParticipants();
	} catch (e) {
		console.error('Failed to join voice:', e);
		connecting = false;
		throw e;
	}
}

async function leave() {
	if (room) {
		room.remoteParticipants.forEach((p) => {
			p.audioTrackPublications.forEach((pub) => {
				if (pub.track) {
					pub.track.detach().forEach((el) => el.remove());
				}
			});
		});
		room.disconnect();
		room = null;
	}
	connected = false;
	connecting = false;
	currentChannelId = null;
	participants = [];
	isMuted = false;
}

async function toggleMute() {
	if (!room) return;

	const newMuted = !isMuted;
	await room.localParticipant.setMicrophoneEnabled(!newMuted);
	isMuted = newMuted;
	updateParticipants();
}

export const voice = {
	get connected() {
		return connected;
	},
	get connecting() {
		return connecting;
	},
	get currentChannelId() {
		return currentChannelId;
	},
	get participants() {
		return participants;
	},
	get isMuted() {
		return isMuted;
	},
	join,
	leave,
	toggleMute
};