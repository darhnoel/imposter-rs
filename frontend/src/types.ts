export type GamePhase = "LOBBY" | "IN_PROGRESS" | "COMPLETED";
export type GameRole = "CREW" | "IMPOSTER";

export interface PlayerState {
  id: string;
  nickname: string;
  isAdmin: boolean;
  connected: boolean;
}

export interface RoomView {
  code: string;
  category: string | null;
  phase: GamePhase;
  players: PlayerState[];
}

export interface PublicRoomSummary {
  code: string;
  hostNickname: string;
  phase: GamePhase;
  category: string | null;
  connectedPlayers: number;
  totalPlayers: number;
  joinable: boolean;
}

export interface TurnState {
  round: number;
  currentTurnIndex: number;
  currentPlayerId: string;
}

export interface GameResult {
  winner: "CREW" | "IMPOSTER";
  guessedPlayerId: string | null;
  imposterPlayerId: string;
}

export interface SuspicionState {
  playerId: string;
  guessedPlayerId: string;
}

export interface GameSnapshot {
  room: RoomView;
  turn: TurnState | null;
  suspicions: SuspicionState[];
  result: GameResult | null;
}

export interface PrivateRoleView {
  gameRole: GameRole;
  category: string;
  topicId: string | null;
}

export interface ChatMessage {
  id: string;
  roomCode: string;
  senderPlayerId: string;
  senderNickname: string;
  text: string;
  createdAt: string;
}

export interface WsError {
  code: string;
  message: string;
}
