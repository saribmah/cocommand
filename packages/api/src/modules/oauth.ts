import type { Transport } from "../transport";

// ---------- Types ----------

export interface PKCEClientOptions {
  providerName: string;
}

export interface AuthorizationRequestOptions {
  endpoint: string;
  clientId: string;
  scope?: string;
  extraParams?: Record<string, string>;
}

export interface AuthorizationRequest {
  authorizationUrl: string;
  codeVerifier: string;
  state: string;
  redirectUri: string;
}

export interface AuthorizationResponse {
  authorizationCode: string;
}

export interface TokenSetOptions {
  accessToken: string;
  refreshToken?: string;
  expiresIn?: number;
  scope?: string;
  idToken?: string;
}

export interface TokenSet extends TokenSetOptions {
  createdAt: number;
}

// ---------- PKCE Crypto Helpers ----------

function base64url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function generateCodeVerifier(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return base64url(bytes.buffer);
}

async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const hash = await crypto.subtle.digest("SHA-256", data);
  return base64url(hash);
}

function generateState(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return base64url(bytes.buffer);
}

// ---------- Helper ----------

export function isTokenExpired(tokenSet: TokenSet): boolean {
  if (!tokenSet.expiresIn) return false;
  const now = Math.floor(Date.now() / 1000);
  return now >= tokenSet.createdAt + tokenSet.expiresIn;
}

// ---------- PKCEClient ----------

interface StartFlowResponse {
  redirect_uri: string;
}

interface PollResponse {
  status: string;
  authorization_code?: string;
}

export class PKCEClient {
  private transport: Transport;
  private extensionId: string;
  private providerName: string;

  constructor(transport: Transport, extensionId: string, providerName: string) {
    this.transport = transport;
    this.extensionId = extensionId;
    this.providerName = providerName;
  }

  async authorizationRequest(
    opts: AuthorizationRequestOptions,
  ): Promise<AuthorizationRequest> {
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateState();

    const { redirect_uri: redirectUri } =
      await this.transport.apiPost<StartFlowResponse>("/oauth/start", {
        state,
      });

    const params = new URLSearchParams({
      client_id: opts.clientId,
      response_type: "code",
      redirect_uri: redirectUri,
      code_challenge: codeChallenge,
      code_challenge_method: "S256",
      state,
      ...(opts.scope ? { scope: opts.scope } : {}),
      ...(opts.extraParams ?? {}),
    });

    const authorizationUrl = `${opts.endpoint}?${params.toString()}`;

    return { authorizationUrl, codeVerifier, state, redirectUri };
  }

  async authorize(request: AuthorizationRequest): Promise<AuthorizationResponse> {
    window.open(request.authorizationUrl, "_blank");

    const deadline = Date.now() + 5 * 60 * 1000; // 5 min timeout
    while (Date.now() < deadline) {
      const res = await this.transport.apiGet<PollResponse>(
        `/oauth/poll?state=${encodeURIComponent(request.state)}&wait=25`,
      );
      if (res.status === "completed" && res.authorization_code) {
        return { authorizationCode: res.authorization_code };
      }
    }

    throw new Error("OAuth authorization timed out after 5 minutes");
  }

  async setTokens(opts: TokenSetOptions): Promise<void> {
    const body = {
      access_token: opts.accessToken,
      refresh_token: opts.refreshToken ?? null,
      expires_in: opts.expiresIn ?? null,
      scope: opts.scope ?? null,
      id_token: opts.idToken ?? null,
      created_at: Math.floor(Date.now() / 1000),
    };
    await this.transport.apiPost(
      `/oauth/tokens/${encodeURIComponent(this.extensionId)}/${encodeURIComponent(this.providerName)}`,
      body,
    );
  }

  async getTokens(): Promise<TokenSet | null> {
    try {
      const raw = await this.transport.apiGet<Record<string, unknown>>(
        `/oauth/tokens/${encodeURIComponent(this.extensionId)}/${encodeURIComponent(this.providerName)}`,
      );
      return {
        accessToken: raw.access_token as string,
        refreshToken: (raw.refresh_token as string) ?? undefined,
        expiresIn: (raw.expires_in as number) ?? undefined,
        scope: (raw.scope as string) ?? undefined,
        idToken: (raw.id_token as string) ?? undefined,
        createdAt: raw.created_at as number,
      };
    } catch {
      return null;
    }
  }

  async removeTokens(): Promise<void> {
    await this.transport.apiDelete(
      `/oauth/tokens/${encodeURIComponent(this.extensionId)}/${encodeURIComponent(this.providerName)}`,
    );
  }
}

// ---------- OAuthApi ----------

export interface OAuthApi {
  createClient(options: PKCEClientOptions): PKCEClient;
}

export function createOAuth(transport: Transport, extensionId: string): OAuthApi {
  return {
    createClient(options: PKCEClientOptions): PKCEClient {
      return new PKCEClient(transport, extensionId, options.providerName);
    },
  };
}
