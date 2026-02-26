import type {AuthResponse, User} from '$lib/types/models';

const API_BASE = 'http://localhost:3000';

export interface GuildResponse {
    id: string;
    name: string;
    icon_url: string | null;
    owner_id: string;
}

export interface ChannelResponse {
    id: string;
    guild_id: string;
    name: string;
    kind: string;
    position: number;
}

export interface AttachmentData {
    filename: string;
    content_type: string;
    size: number;
    url: string;
}

export interface MessageResponse {
    id: string;
    channel_id: string;
    author_id: string;
    content: string;
    attachments: AttachmentData[];
    timestamp: number;
    edited_timestamp: number | null;
}

export interface UploadResponse {
    id: string;
    filename: string;
    content_type: string;
    size: number;
    url: string;
}

export interface InviteResponse {
    code: string;
    guild_id: string;
    guild_name: string;
    creator_id: string;
    max_uses: number | null;
    uses: number;
    expires_at: string | null;
}

export interface VoiceTokenResponse {
    token: string;
    url: string;
}

class ApiService {
    private token: string | null = null;

    setToken(token: string) {
        this.token = token;
    }

    clearToken() {
        this.token = null;
    }

    private async request<T>(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<T> {
        const headers: Record<string, string> = {
            'Content-Type': 'application/json',
            ...((options.headers as Record<string, string>) || {})
        };

        if (this.token) {
            headers['Authorization'] = `Bearer ${this.token}`;
        }

        const response = await fetch(`${API_BASE}${endpoint}`, {
            ...options,
            headers
        });

        if (!response.ok) {
            const error = await response.json().catch(() => ({
                error: 'Unknown error'
            }));
            throw new Error(error.error || `HTTP ${response.status}`);
        }

        return response.json();
    }

    async register(username: string, email: string, password: string): Promise<AuthResponse> {
        const res = await this.request<AuthResponse>('/auth/register', {
            method: 'POST',
            body: JSON.stringify({username, email, password})
        });
        this.setToken(res.token);
        return res;
    }

    async login(email: string, password: string): Promise<AuthResponse> {
        const res = await this.request<AuthResponse>('/auth/login', {
            method: 'POST',
            body: JSON.stringify({email, password})
        });
        this.setToken(res.token);
        return res;
    }

    async getMe(): Promise<User> {
        return this.request('/users/me');
    }

    // Guilds
    async createGuild(name: string): Promise<GuildResponse> {
        return this.request('/guilds', {
            method: 'POST',
            body: JSON.stringify({name})
        });
    }

    async listGuilds(): Promise<GuildResponse[]> {
        return this.request('/guilds');
    }

    // Channels
    async createChannel(guildId: string, name: string, kind?: string): Promise<ChannelResponse> {
        return this.request(`/guilds/${guildId}/channels`, {
            method: 'POST',
            body: JSON.stringify({name, kind})
        });
    }

    async listChannels(guildId: string): Promise<ChannelResponse[]> {
        return this.request(`/guilds/${guildId}/channels`);
    }

    // Messages
    async listMessages(channelId: string, limit?: number, before?: string): Promise<MessageResponse[]> {
        const params = new URLSearchParams();
        if (limit) params.set('limit', limit.toString());
        if (before) params.set('before', before);
        return this.request(`/channels/${channelId}/messages?${params}`);
    }

    async uploadFile(channelId: string, file: File): Promise<UploadResponse> {
        const formData = new FormData();
        formData.append('channel_id', channelId);
        formData.append('file', file);

        const headers: Record<string, string> = {};
        if (this.token) {
            headers['Authorization'] = `Bearer ${this.token}`;
        }

        const res = await fetch(`${API_BASE}/upload`, {
            method: 'POST',
            headers,
            body: formData
        });

        if (!res.ok) {
            const err = await res.json().catch(() => ({error: 'Upload failed'}));
            throw new Error(err.error || 'Upload failed');
        }

        return res.json();
    }

    // Invites
    async createInvite(guildId: string, maxUses?: number, maxAge?: number): Promise<InviteResponse> {
        return this.request(`/guilds/${guildId}/invites`, {
            method: 'POST',
            body: JSON.stringify({max_uses: maxUses ?? null, max_age: maxAge ?? null})
        });
    }

    async getInvite(code: string): Promise<InviteResponse> {
        return this.request(`/invites/${code}`);
    }

    async joinInvite(code: string): Promise<InviteResponse> {
        return this.request(`/invites/${code}/join`, {
            method: 'POST'
        });
    }

    // Voice
    async getVoiceToken(channelId: string): Promise<VoiceTokenResponse> {
        return this.request('/voice/token', {
            method: 'POST',
            body: JSON.stringify({channel_id: channelId})
        });
    }
}

export const api = new ApiService();