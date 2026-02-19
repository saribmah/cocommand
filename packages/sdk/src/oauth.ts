import type { Client } from "@cocommand/api";
import {
  startFlow,
  pollFlow,
  getTokens,
  setTokens,
  deleteTokens,
  type PollResponse,
  type StartFlowResponse,
} from "@cocommand/api";
import { SdkError } from "./errors";
import { unwrapApiResponse } from "./request";

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

export function isTokenExpired(tokenSet: TokenSet): boolean {
  if (!tokenSet.expiresIn) return false;
  const now = Math.floor(Date.now() / 1000);
  return now >= tokenSet.createdAt + tokenSet.expiresIn;
}

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

    const start = await startFlow({
      client: this.client,
      body: { state },
    });
    const data = unwrapApiResponse<StartFlowResponse>("oauth.startFlow", start);

    const redirectUri = data.redirect_uri;

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

    const deadline = Date.now() + 5 * 60 * 1000;
    while (Date.now() < deadline) {
      const poll = await pollFlow({
        client: this.client,
        query: { state: request.state, wait: 25 },
      });

      if (poll.error) {
        continue;
      }

      const res = poll.data as PollResponse | undefined;
      if (res?.status === "completed" && res.authorization_code) {
        return { authorizationCode: res.authorization_code };
      }
    }

    throw new SdkError({
      code: "timeout",
      message: "OAuth authorization timed out after 5 minutes",
      source: "oauth.authorize",
    });
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

    const result = await setTokens({
      client: this.client,
      path: { ext: this.extensionId, provider: this.providerName },
      body,
    });

    unwrapApiResponse("oauth.setTokens", result, { allowNull: true });
  }

  async getTokensValue(): Promise<TokenSet | null> {
    const result = await getTokens({
      client: this.client,
      path: { ext: this.extensionId, provider: this.providerName },
    });

    if (result.error || !result.response.ok || !result.data) {
      return null;
    }

    const raw = result.data as Record<string, unknown>;
    return {
      accessToken: raw.access_token as string,
      refreshToken: (raw.refresh_token as string | null) ?? undefined,
      expiresIn: (raw.expires_in as number | null) ?? undefined,
      scope: (raw.scope as string | null) ?? undefined,
      idToken: (raw.id_token as string | null) ?? undefined,
      createdAt: raw.created_at as number,
    };
  }

  async removeTokens(): Promise<void> {
    const result = await deleteTokens({
      client: this.client,
      path: { ext: this.extensionId, provider: this.providerName },
    });
    unwrapApiResponse("oauth.deleteTokens", result, { allowNull: true });
  }
}

export interface OAuthApi {
  createClient(options: PKCEClientOptions): PKCEClient;
}

export function createOAuthApi(client: Client, extensionId: string): OAuthApi {
  return {
    createClient(options: PKCEClientOptions): PKCEClient {
      return new PKCEClient(client, extensionId, options.providerName);
    },
  };
}
