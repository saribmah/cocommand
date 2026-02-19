import type { Client } from "../client";
import {
  startFlow,
  pollFlow,
  getTokens,
  setTokens,
  deleteTokens,
  type StartFlowResponse,
  type PollResponse,
} from "@cocommand/api";

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

export class PKCEClient {
  private client: Client;
  private extensionId: string;
  private providerName: string;

  constructor(client: Client, extensionId: string, providerName: string) {
    this.client = client;
    this.extensionId = extensionId;
    this.providerName = providerName;
  }

  async authorizationRequest(
    opts: AuthorizationRequestOptions,
  ): Promise<AuthorizationRequest> {
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateState();

    const { data, error } = await startFlow({
      client: this.client,
      body: { state },
    });

    if (error || !data) {
      throw new Error("Failed to start OAuth flow");
    }

    const redirectUri = (data as StartFlowResponse).redirect_uri;

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
      const { data, error } = await pollFlow({
        client: this.client,
        query: { state: request.state, wait: 25 },
      });

      if (error) continue;

      const res = data as PollResponse | undefined;
      if (res?.status === "completed" && res.authorization_code) {
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
    const { error } = await setTokens({
      client: this.client,
      path: { ext: this.extensionId, provider: this.providerName },
      body,
    });
    if (error) {
      throw new Error("Failed to store tokens");
    }
  }

  async getTokens(): Promise<TokenSet | null> {
    try {
      const { data, error } = await getTokens({
        client: this.client,
        path: { ext: this.extensionId, provider: this.providerName },
      });
      if (error || !data) return null;
      const raw = data as Record<string, unknown>;
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
    const { error } = await deleteTokens({
      client: this.client,
      path: { ext: this.extensionId, provider: this.providerName },
    });
    if (error) {
      throw new Error("Failed to delete tokens");
    }
  }
}

// ---------- OAuthApi ----------

export interface OAuthApi {
  createClient(options: PKCEClientOptions): PKCEClient;
}

export function createOAuth(client: Client, extensionId: string): OAuthApi {
  return {
    createClient(options: PKCEClientOptions): PKCEClient {
      return new PKCEClient(client, extensionId, options.providerName);
    },
  };
}
