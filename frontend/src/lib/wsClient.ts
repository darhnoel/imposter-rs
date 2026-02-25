import type {
  ChatMessage,
  GameResult,
  GameSnapshot,
  PrivateRoleView,
  PublicRoomSummary,
  RoomView,
  SuspicionState,
  TurnState,
  WsError,
} from "../types";

type PendingRequest = {
  resolve: (data: unknown) => void;
  reject: (err: WsError) => void;
};

type EventCallback = (snapshot: GameSnapshot) => void;
type ChatCallback = (roomCode: string, message: ChatMessage) => void;

type Envelope = {
  id: string;
  op: string;
  payload: Record<string, unknown>;
  token: string | null;
};

export class WsProtocolClient {
  private ws: WebSocket | null = null;
  private reqId = 0;
  private pending = new Map<string, PendingRequest>();
  private gameUpdatedCallbacks = new Set<EventCallback>();
  private chatCallbacks = new Set<ChatCallback>();

  async connect(url: string): Promise<void> {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return;
    }
    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(url);
      this.ws = ws;
      ws.onopen = () => resolve();
      ws.onerror = () => reject(new Error("websocket connection failed"));
      ws.onclose = () => {
        if (this.ws === ws) {
          this.ws = null;
        }
      };
      ws.onmessage = (message) => this.handleMessage(String(message.data));
    });
  }

  onGameUpdated(cb: EventCallback): () => void {
    this.gameUpdatedCallbacks.add(cb);
    return () => this.gameUpdatedCallbacks.delete(cb);
  }

  onChatMessage(cb: ChatCallback): () => void {
    this.chatCallbacks.add(cb);
    return () => this.chatCallbacks.delete(cb);
  }

  close(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }

  async categories(): Promise<string[]> {
    return this.request("categories", {});
  }

  async listRooms(): Promise<PublicRoomSummary[]> {
    return this.request("listRooms", {});
  }

  async gameSnapshot(roomCode: string, token: string | null): Promise<GameSnapshot> {
    return this.request("gameSnapshot", { roomCode }, token);
  }

  async myRole(roomCode: string, token: string): Promise<PrivateRoleView> {
    return this.request("myRole", { roomCode }, token);
  }

  async createRoom(
    code: string,
    nickname: string,
    isPublic = false,
  ): Promise<{ room: RoomView; token: string }> {
    return this.request("createRoom", { code, nickname, public: isPublic });
  }

  async joinRoom(code: string, nickname: string): Promise<{ room: RoomView; token: string }> {
    return this.request("joinRoom", { code, nickname });
  }

  async leaveRoom(code: string, token: string): Promise<RoomView> {
    return this.request("leaveRoom", { code }, token);
  }

  async setCategory(code: string, category: string, token: string): Promise<RoomView> {
    return this.request("setCategory", { code, category }, token);
  }

  async startGame(code: string, token: string): Promise<GameSnapshot> {
    return this.request("startGame", { code }, token);
  }

  async nextTurn(code: string, token: string): Promise<TurnState> {
    return this.request("nextTurn", { code }, token);
  }

  async guessImposter(code: string, guessedPlayerId: string, token: string): Promise<SuspicionState> {
    return this.request("guessImposter", { code, guessedPlayerId }, token);
  }

  async revealResult(code: string, token: string): Promise<GameResult> {
    return this.request("revealResult", { code }, token);
  }

  async restartGame(code: string, token: string): Promise<GameSnapshot> {
    return this.request("restartGame", { code }, token);
  }

  async endGame(code: string, token: string): Promise<GameSnapshot> {
    return this.request("endGame", { code }, token);
  }

  async chatHistory(code: string, token: string): Promise<ChatMessage[]> {
    return this.request("chatHistory", { code }, token);
  }

  async sendChat(code: string, text: string, token: string): Promise<ChatMessage> {
    return this.request("sendChat", { code, text }, token);
  }

  private async request<T>(op: string, payload: Record<string, unknown>, token?: string | null): Promise<T> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("websocket is not connected");
    }
    this.reqId += 1;
    const id = String(this.reqId);

    const envelope: Envelope = {
      id,
      op,
      payload,
      token: token ?? null,
    };

    const promise = new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: (data) => resolve(data as T), reject });
    });

    this.ws.send(JSON.stringify(envelope));
    return promise;
  }

  private handleMessage(raw: string): void {
    const msg = JSON.parse(raw) as Record<string, unknown>;
    const msgType = msg.type;
    if (msgType === "response") {
      const id = String(msg.id ?? "");
      const pending = this.pending.get(id);
      if (!pending) {
        return;
      }
      this.pending.delete(id);
      if (msg.ok === true) {
        pending.resolve(msg.data);
      } else {
        pending.reject((msg.error as WsError) ?? { code: "UnknownError", message: "request failed" });
      }
      return;
    }

    if (msgType === "event" && msg.event === "gameUpdated") {
      const snapshot = msg.snapshot as GameSnapshot;
      for (const cb of this.gameUpdatedCallbacks) {
        cb(snapshot);
      }
      return;
    }

    if (msgType === "event" && msg.event === "chatMessage") {
      const roomCode = String(msg.code ?? "");
      const message = msg.message as ChatMessage;
      for (const cb of this.chatCallbacks) {
        cb(roomCode, message);
      }
    }
  }
}
