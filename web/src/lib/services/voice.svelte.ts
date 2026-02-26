import {type LocalParticipant, type Participant, type RemoteParticipant, Room, RoomEvent, Track} from 'livekit-client';
import {api} from '$lib/services/api';

let room: Room | null = $state(null);
let connected = $state(false);
let connecting = $state(false);
let currentChannelId: string | null = $state(null);
let participants: ParticipantInfo[] = $state([]);
let isMuted = $state(false);
let isCameraOn = $state(false);
let isScreenSharing = $state(false);

let updateTimeout: ReturnType<typeof setTimeout> | null = null;

function scheduleUpdate() {
    if (updateTimeout) return;
    updateTimeout = setTimeout(() => {
        updateTimeout = null;
        updateParticipants();
    }, 50);
}

export interface TrackInfo {
    sid: string;
    kind: 'video' | 'screen';
    element: HTMLVideoElement | null;
}

export interface ParticipantInfo {
    identity: string;
    name: string;
    isSpeaking: boolean;
    isMuted: boolean;
    isLocal: boolean;
    videoTracks: TrackInfo[];
}

function getTrackInfos(participant: Participant): TrackInfo[] {
    const tracks: TrackInfo[] = [];

    participant.videoTrackPublications.forEach((pub) => {
        if (pub.track && pub.track.kind === Track.Kind.Video) {
            const isScreen = pub.source === Track.Source.ScreenShare;
            tracks.push({
                sid: pub.track.sid,
                kind: isScreen ? 'screen' : 'video',
                element: null
            });
        }
    });

    return tracks;
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
        isLocal: true,
        videoTracks: getTrackInfos(local)
    });

    room.remoteParticipants.forEach((p: RemoteParticipant) => {
        list.push({
            identity: p.identity,
            name: p.name ?? p.identity,
            isSpeaking: p.isSpeaking,
            isMuted: !p.isMicrophoneEnabled,
            isLocal: false,
            videoTracks: getTrackInfos(p)
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
            dynacast: true
        });

        newRoom.on(RoomEvent.TrackSubscribed, (track, _publication, _participant) => {
            if (track.kind === Track.Kind.Audio) {
                const el = track.attach();
                el.id = `lk-audio-${track.sid}`;
                document.body.appendChild(el);
            }
            scheduleUpdate();
        });

        newRoom.on(RoomEvent.TrackUnsubscribed, (track) => {
            track.detach().forEach((el) => el.remove());
            scheduleUpdate();
        });

        newRoom.on(RoomEvent.ParticipantConnected, () => scheduleUpdate());
        newRoom.on(RoomEvent.ParticipantDisconnected, () => scheduleUpdate());
        newRoom.on(RoomEvent.ActiveSpeakersChanged, () => scheduleUpdate());
        newRoom.on(RoomEvent.TrackMuted, () => scheduleUpdate());
        newRoom.on(RoomEvent.TrackUnmuted, () => scheduleUpdate());
        newRoom.on(RoomEvent.LocalTrackPublished, () => scheduleUpdate());
        newRoom.on(RoomEvent.LocalTrackUnpublished, () => scheduleUpdate());
        newRoom.on(RoomEvent.TrackPublished, () => scheduleUpdate());
        newRoom.on(RoomEvent.Disconnected, () => {
            connected = false;
            currentChannelId = null;
            participants = [];
            isCameraOn = false;
            isScreenSharing = false;
        });

        await newRoom.connect(res.url, res.token);
        await newRoom.localParticipant.setMicrophoneEnabled(true);

        room = newRoom;
        connected = true;
        connecting = false;
        currentChannelId = channelId;
        isMuted = false;
        isCameraOn = false;
        isScreenSharing = false;

        updateParticipants();
    } catch (e) {
        console.error('Failed to join voice:', e);
        connecting = false;
        throw e;
    }
}

async function leave() {
    if (updateTimeout) {
        clearTimeout(updateTimeout);
        updateTimeout = null;
    }
    if (room) {
        room.remoteParticipants.forEach((p) => {
            p.audioTrackPublications.forEach((pub) => {
                if (pub.track) pub.track.detach().forEach((el) => el.remove());
            });
            p.videoTrackPublications.forEach((pub) => {
                if (pub.track) pub.track.detach().forEach((el) => el.remove());
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
    isCameraOn = false;
    isScreenSharing = false;
}

async function toggleMute() {
    if (!room) return;
    const newMuted = !isMuted;
    await room.localParticipant.setMicrophoneEnabled(!newMuted);
    isMuted = newMuted;
    updateParticipants();
}

async function toggleCamera() {
    if (!room) return;
    const newState = !isCameraOn;
    await room.localParticipant.setCameraEnabled(newState, {
        resolution: {width: 1280, height: 720, frameRate: 30}
    });
    isCameraOn = newState;
    updateParticipants();
}

async function toggleScreenShare() {
    if (!room) return;
    const newState = !isScreenSharing;
    try {
        await room.localParticipant.setScreenShareEnabled(newState, {
            contentHint: 'motion',
            resolution: {width: 1920, height: 1080, frameRate: 30},
            audio: true,
            selfBrowserSurface: 'exclude',
            surfaceSwitching: 'include'
        }, {
            videoEncoding: {
                maxBitrate: 5_000_000,
                maxFramerate: 30
            },
            screenShareEncoding: {
                maxBitrate: 5_000_000,
                maxFramerate: 30
            }
        });
        isScreenSharing = newState;
    } catch (e) {
        console.warn('Screen share toggle failed:', e);
        isScreenSharing = false;
    }
    updateParticipants();
}

function attachTrack(trackSid: string, element: HTMLVideoElement) {
    if (!room) return;

    const allParticipants: Participant[] = [room.localParticipant, ...room.remoteParticipants.values()];

    for (const p of allParticipants) {
        for (const pub of p.videoTrackPublications.values()) {
            if (pub.track && pub.track.sid === trackSid) {
                pub.track.attach(element);
                return;
            }
        }
        if (p === room.localParticipant) {
            for (const pub of (p as LocalParticipant).videoTrackPublications.values()) {
                if (pub.trackSid === trackSid && pub.track) {
                    pub.track.attach(element);
                    return;
                }
            }
        }
    }
}

function detachTrack(trackSid: string) {
    if (!room) return;

    const allParticipants = [room.localParticipant, ...room.remoteParticipants.values()];

    for (const p of allParticipants) {
        for (const pub of p.videoTrackPublications.values()) {
            if (pub.track && pub.track.sid === trackSid) {
                pub.track.detach().forEach((el) => el.remove());
                return;
            }
        }
    }
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
    get isCameraOn() {
        return isCameraOn;
    },
    get isScreenSharing() {
        return isScreenSharing;
    },
    join,
    leave,
    toggleMute,
    toggleCamera,
    toggleScreenShare,
    attachTrack,
    detachTrack
};