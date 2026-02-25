import { useEffect, useMemo, useRef, useState } from "react";
import { WsProtocolClient } from "./lib/wsClient";
import type {
  ChatMessage,
  GameSnapshot,
  PrivateRoleView,
  PublicRoomSummary,
  RoomView,
  WsError,
} from "./types";

const wsUrl = import.meta.env.VITE_WS_URL ?? "ws://127.0.0.1:4000/ws";

function isForbidden(error: unknown): boolean {
  const e = error as WsError;
  return e?.code === "Forbidden";
}

function findLocalPlayerId(room: RoomView, nickname: string, fallbackAdmin = false): string | null {
  const normalized = nickname.trim().toLowerCase();
  const matched = room.players.find(
    (p) => p.connected && p.nickname.trim().toLowerCase() === normalized,
  );
  if (matched) {
    return matched.id;
  }
  if (fallbackAdmin) {
    return room.players.find((p) => p.isAdmin)?.id ?? null;
  }
  return null;
}

export default function App() {
  const clientRef = useRef(new WsProtocolClient());

  const [connected, setConnected] = useState(false);
  const [status, setStatus] = useState("Connecting...");
  const [statusTone, setStatusTone] = useState<"info" | "success">("info");
  const [error, setError] = useState<string | null>(null);

  const [token, setToken] = useState<string | null>(null);
  const [roomCode, setRoomCode] = useState("");
  const [snapshot, setSnapshot] = useState<GameSnapshot | null>(null);
  const [categories, setCategories] = useState<string[]>([]);
  const [roleView, setRoleView] = useState<PrivateRoleView | null>(null);
  const [roleRevealed, setRoleRevealed] = useState(true);

  const [createNickname, setCreateNickname] = useState("");
  const [createCode, setCreateCode] = useState("");
  const [joinNickname, setJoinNickname] = useState("");
  const [joinSearch, setJoinSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("");
  const [guessPlayerId, setGuessPlayerId] = useState("");
  const [createIsPublic, setCreateIsPublic] = useState(true);
  const [lobbyMode, setLobbyMode] = useState<"admin" | "join">("admin");
  const [publicRooms, setPublicRooms] = useState<PublicRoomSummary[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [showPlayersInProgress, setShowPlayersInProgress] = useState(false);

  const [isAdmin, setIsAdmin] = useState(false);
  const [localPlayerId, setLocalPlayerId] = useState<string | null>(null);
  const roomCodeRef = useRef("");
  const phase = snapshot?.room.phase ?? "LOBBY";
  const roundNumber = snapshot?.turn?.round ?? null;

  useEffect(() => {
    let mounted = true;
    const client = clientRef.current;

    client
      .connect(wsUrl)
      .then(async () => {
        if (!mounted) {
          return;
        }
        setConnected(true);
        setStatusTone("success");
        setStatus("Connected");
        const [list, rooms] = await Promise.all([client.categories(), client.listRooms()]);
        setCategories(list);
        setPublicRooms(rooms);
        if (list.length > 0) {
          setSelectedCategory((prev) => prev || list[0]);
        }
      })
      .catch((err: unknown) => {
        if (!mounted) {
          return;
        }
        setStatusTone("info");
        setError(String(err));
        setStatus("Disconnected");
      });

    const unsubscribe = client.onGameUpdated((next) => {
      if (!mounted) {
        return;
      }
      setSnapshot(next);
    });
    const unsubscribeChat = client.onChatMessage((eventRoomCode, message) => {
      if (!mounted || roomCodeRef.current !== eventRoomCode) {
        return;
      }
      setChatMessages((prev) => (prev.some((m) => m.id === message.id) ? prev : [...prev, message]));
    });

    return () => {
      mounted = false;
      unsubscribe();
      unsubscribeChat();
      client.close();
    };
  }, []);

  useEffect(() => {
    roomCodeRef.current = roomCode;
  }, [roomCode]);

  useEffect(() => {
    if (!roleRevealed || phase !== "IN_PROGRESS") {
      setRoleView(null);
      return;
    }
    if (!roomCode || !token) {
      return;
    }
    let active = true;
    clientRef.current
      .myRole(roomCode, token)
      .then((role) => {
        if (active) {
          setRoleView(role);
        }
      })
      .catch((err: unknown) => {
        if (active) {
          setError(`myRole failed: ${String((err as WsError).message ?? err)}`);
        }
      });
    return () => {
      active = false;
    };
  }, [roleRevealed, phase, roomCode, token, roundNumber]);

  useEffect(() => {
    if (phase === "IN_PROGRESS") {
      setShowPlayersInProgress(false);
    }
  }, [phase, roomCode]);

  const connectedPlayers = useMemo(() => {
    return (snapshot?.room.players ?? []).filter((p) => p.connected);
  }, [snapshot]);
  const playerById = useMemo(() => {
    const map = new Map<string, { id: string; nickname: string }>();
    for (const p of snapshot?.room.players ?? []) {
      map.set(p.id, { id: p.id, nickname: p.nickname });
    }
    return map;
  }, [snapshot]);
  const suspicionByPlayerId = useMemo(() => {
    const map = new Map<string, string>();
    for (const suspicion of snapshot?.suspicions ?? []) {
      map.set(suspicion.playerId, suspicion.guessedPlayerId);
    }
    return map;
  }, [snapshot]);
  const joinablePublicRooms = useMemo(
    () => publicRooms.filter((room) => room.joinable),
    [publicRooms],
  );
  const filteredPublicRooms = useMemo(() => {
    const q = joinSearch.trim().toLowerCase();
    if (!q) {
      return joinablePublicRooms;
    }
    return joinablePublicRooms.filter(
      (room) =>
        room.code.toLowerCase().includes(q) || room.hostNickname.toLowerCase().includes(q),
    );
  }, [joinablePublicRooms, joinSearch]);
  const isInRoom = Boolean(token && roomCode);
  const turnPlayerId = snapshot?.turn?.currentPlayerId ?? "-";
  const localIdentity = isInRoom ? (isAdmin ? "ADMIN" : "PLAYER") : "NOT IN ROOM";
  const phaseLabel =
    phase === "IN_PROGRESS"
      ? "Round is live"
      : phase === "COMPLETED"
        ? "Round finished"
        : "Lobby setup";
  const showLobbyPanel = !isInRoom;
  const showMatchState = isInRoom;
  const showPlayersPanel = isInRoom && (phase !== "IN_PROGRESS" || showPlayersInProgress);
  const showAdminControls = isInRoom && phase === "LOBBY";
  const showGamePanel = isInRoom && phase === "IN_PROGRESS";
  const showTurnBoard = isInRoom && phase === "IN_PROGRESS";
  const showSuspicionsPanel = isInRoom && phase === "IN_PROGRESS";
  const showChatPanel = isInRoom && phase === "IN_PROGRESS";
  const showRoleCard = isInRoom && phase === "IN_PROGRESS";
  const showResultPanel = isInRoom && phase === "COMPLETED";
  const appliedCategory = snapshot?.room.category ?? null;
  const selectedAlreadyApplied = Boolean(appliedCategory && appliedCategory === selectedCategory);
  const canChatThisTurn =
    phase === "IN_PROGRESS" && Boolean(localPlayerId) && localPlayerId === turnPlayerId;

  async function refreshChatHistory(targetCode = roomCode, targetToken = token) {
    if (!targetCode || !targetToken) {
      return;
    }
    try {
      const messages = await clientRef.current.chatHistory(targetCode, targetToken);
      setChatMessages(messages);
    } catch (err) {
      setError(`chatHistory failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function refreshSnapshot() {
    if (!roomCode) {
      return;
    }
    try {
      const next = await clientRef.current.gameSnapshot(roomCode, token);
      setSnapshot(next);
    } catch (err) {
      setError(`gameSnapshot failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function refreshPublicRooms() {
    try {
      const rooms = await clientRef.current.listRooms();
      setPublicRooms(rooms);
    } catch (err) {
      setError(`listRooms failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function handleCreateRoom() {
    setError(null);
    const nickname = createNickname.trim();
    const code = createCode.trim();
    if (!nickname || !code) {
      setError("createRoom failed: nickname and room code are required");
      return;
    }
    try {
      const { room, token: sessionToken } = await clientRef.current.createRoom(
        code,
        nickname,
        createIsPublic,
      );
      setRoomCode(room.code);
      setToken(sessionToken);
      setIsAdmin(true);
      setLocalPlayerId(findLocalPlayerId(room, nickname, true));
      setRoleView(null);
      setRoleRevealed(true);
      setStatusTone("success");
      setStatus(`Room ${room.code} created`);
      await refreshSnapshot();
    } catch (err) {
      setError(`createRoom failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function handleJoinRoom(code: string) {
    setError(null);
    const nickname = joinNickname.trim();
    const normalizedCode = code.trim();
    if (!nickname) {
      setError("joinRoom failed: nickname is required");
      return;
    }
    if (!normalizedCode) {
      setError("joinRoom failed: room code is required");
      return;
    }
    try {
      const { room, token: sessionToken } = await clientRef.current.joinRoom(normalizedCode, nickname);
      setRoomCode(room.code);
      setToken(sessionToken);
      setIsAdmin(false);
      setLocalPlayerId(findLocalPlayerId(room, nickname));
      setRoleView(null);
      setRoleRevealed(true);
      setStatusTone("success");
      setStatus(`Joined room ${room.code}`);
      await refreshSnapshot();
    } catch (err) {
      setError(`joinRoom failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function handleLeaveRoom() {
    if (!roomCode || !token) {
      return;
    }
    setError(null);
    try {
      await clientRef.current.leaveRoom(roomCode, token);
      setStatusTone("info");
      setStatus("Left room");
      setRoomCode("");
      setToken(null);
      setSnapshot(null);
      setRoleView(null);
      setRoleRevealed(true);
      setIsAdmin(false);
      setLocalPlayerId(null);
      setChatMessages([]);
      setChatInput("");
      setShowPlayersInProgress(false);
      void refreshPublicRooms();
    } catch (err) {
      setError(`leaveRoom failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function runAdminAction(fn: () => Promise<unknown>, label: string) {
    setError(null);
    try {
      await fn();
      setStatusTone("success");
      setStatus(`${label} success`);
      await refreshSnapshot();
    } catch (err) {
      if (isForbidden(err)) {
        setIsAdmin(false);
      }
      setError(`${label} failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  async function handleGuessImposter() {
    if (!roomCode || !token) {
      return;
    }
    const guessedPlayerId = guessPlayerId.trim();
    if (!guessedPlayerId) {
      setError("guessImposter failed: player id is required");
      return;
    }
    await runAdminAction(
      () => clientRef.current.guessImposter(roomCode, guessedPlayerId, token),
      "guessImposter",
    );
  }

  async function handleSendChat() {
    if (!roomCode || !token) {
      return;
    }
    const text = chatInput.trim();
    if (!text) {
      setError("sendChat failed: message is required");
      return;
    }
    setError(null);
    try {
      await clientRef.current.sendChat(roomCode, text, token);
      setChatInput("");
      setStatusTone("success");
      setStatus("chat sent");
    } catch (err) {
      setError(`sendChat failed: ${String((err as WsError).message ?? err)}`);
    }
  }

  useEffect(() => {
    if (isInRoom) {
      return;
    }
    let active = true;
    const tick = async () => {
      try {
        const rooms = await clientRef.current.listRooms();
        if (active) {
          setPublicRooms(rooms);
        }
      } catch {
        // keep polling even if one request fails
      }
    };
    void tick();
    const interval = window.setInterval(() => {
      void tick();
    }, 3000);
    return () => {
      active = false;
      window.clearInterval(interval);
    };
  }, [isInRoom]);

  useEffect(() => {
    if (phase !== "IN_PROGRESS") {
      return;
    }
    void refreshChatHistory();
  }, [phase, roomCode, token]);

  return (
    <main className={`app ${showLobbyPanel ? "app-lobby" : "app-game"}`}>
      <header className="banner" data-testid="connection-banner" data-phase={phase}>
        <h1 className="title">Imposter</h1>
        <span className={`pill ${connected ? "pill-live" : "pill-down"}`}>
          WS: {connected ? "connected" : "disconnected"}
        </span>
        <span className="pill">Room: {roomCode || "-"}</span>
        <span className={`pill ${phase === "IN_PROGRESS" ? "pill-live" : "pill-muted"}`}>
          Phase: {phase}
        </span>
      </header>

      {showMatchState && (
        <section className="card match-state compact-card" data-testid="match-state-card">
          <div className="match-state-line">
            <p data-testid="match-line">
              <strong>{phaseLabel}</strong> | You: <strong>{localIdentity}</strong> | Turn:{" "}
              <strong>{turnPlayerId}</strong>
            </p>
            <div className="match-actions">
              {phase === "IN_PROGRESS" && (
                <button
                  data-testid="toggle-players-btn"
                  onClick={() => setShowPlayersInProgress((v) => !v)}
                >
                  {showPlayersInProgress ? "Hide Players" : "Show Players"}
                </button>
              )}
              <button data-testid="leave-room-btn" onClick={handleLeaveRoom}>Leave Room</button>
            </div>
          </div>
        </section>
      )}

      {showLobbyPanel && (
        <section className="card" data-testid="lobby-panel">
          <h2>Lobby</h2>
          <div className="row" data-testid="lobby-mode-switch">
            <button
              data-testid="mode-admin-btn"
              className={lobbyMode === "admin" ? "mode-active" : ""}
              onClick={() => setLobbyMode("admin")}
            >
              Become Admin
            </button>
            <button
              data-testid="mode-join-btn"
              className={lobbyMode === "join" ? "mode-active" : ""}
              onClick={() => setLobbyMode("join")}
            >
              Browse Boards
            </button>
          </div>

          {lobbyMode === "admin" && (
            <>
              <p data-testid="admin-intent-text">Create and manage a board.</p>
              <div className="row lobby-admin-form">
                <input
                  data-testid="create-nickname-input"
                  value={createNickname}
                  onChange={(e) => setCreateNickname(e.target.value)}
                  placeholder="nickname"
                />
                <input
                  data-testid="create-code-input"
                  value={createCode}
                  onChange={(e) => setCreateCode(e.target.value)}
                  placeholder="board code"
                />
                <button data-testid="create-room-btn" onClick={handleCreateRoom}>Create Board</button>
              </div>
              <label className="checkbox-row">
                <input
                  data-testid="create-public-checkbox"
                  type="checkbox"
                  checked={createIsPublic}
                  onChange={(e) => setCreateIsPublic(e.target.checked)}
                />
                Show this board in public list
              </label>
            </>
          )}

          {lobbyMode === "join" && (
            <>
              <p data-testid="join-intent-text">Search and select a public board.</p>
              <div className="row lobby-join-form">
                <input
                  data-testid="join-search-input"
                  value={joinSearch}
                  onChange={(e) => setJoinSearch(e.target.value)}
                  placeholder="search code or host"
                />
                <input
                  data-testid="join-nickname-input"
                  value={joinNickname}
                  onChange={(e) => setJoinNickname(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      const first = filteredPublicRooms[0];
                      if (first) {
                        void handleJoinRoom(first.code);
                      }
                    }
                  }}
                  placeholder="nickname"
                />
              </div>
              <ul className="public-room-list" data-testid="public-room-list">
                {joinablePublicRooms.length === 0 && (
                  <li className="player-empty" data-testid="public-room-empty">No joinable public boards right now.</li>
                )}
                {joinablePublicRooms.length > 0 && filteredPublicRooms.length === 0 && (
                  <li className="player-empty" data-testid="public-room-empty">No boards match this search.</li>
                )}
                {filteredPublicRooms.map((room) => (
                  <li key={room.code} data-testid={`public-room-${room.code}`} className="public-room-row">
                    <span className="public-room-code"><strong>{room.code}</strong></span>
                    <span className="public-room-host">Host: <strong>{room.hostNickname}</strong></span>
                    <span className="public-room-count">{room.connectedPlayers}/{room.totalPlayers}</span>
                    <button
                      data-testid={`join-public-${room.code}`}
                      onClick={() => handleJoinRoom(room.code)}
                    >
                      Join
                    </button>
                  </li>
                ))}
              </ul>
            </>
          )}
        </section>
      )}

      {showPlayersPanel && (
        <section className="card compact-card players-compact" data-testid="players-panel">
        <h2>Players ({connectedPlayers.length})</h2>
        <ul data-testid="player-list" className="player-list">
          {connectedPlayers.length === 0 && <li className="player-empty">No connected players.</li>}
          {connectedPlayers.map((p) => (
            <li
              key={p.id}
              data-testid={`player-${p.id}`}
              className={`player-row player-row-compact ${p.isAdmin ? "player-row-admin" : "player-row-member"}`}
            >
              <span className="player-name">{p.nickname}</span>
              <code className="player-id">{p.id}</code>
              <span
                className={`player-badge ${p.isAdmin ? "badge-admin" : "badge-member"}`}
                data-testid={`role-badge-${p.id}`}
              >
                {p.isAdmin ? "ADMIN" : "PLAYER"}
              </span>
            </li>
          ))}
        </ul>
        </section>
      )}

      {showAdminControls && (
        <section className="card" data-testid="admin-controls">
        <h2>Admin Controls</h2>
        <p data-testid="admin-mode-label">{isAdmin ? "You are admin." : "Admin-only actions locked."}</p>
        <p data-testid="current-category-label">
          Current category: <strong>{appliedCategory ?? "(not set)"}</strong>
        </p>
        <div className="row">
          <select
            data-testid="set-category-select"
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value)}
            disabled={!roomCode || !token}
          >
            {categories.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
          <button
            data-testid="set-category-btn"
            disabled={!isAdmin || !token || !roomCode}
            onClick={() =>
              runAdminAction(
                () => clientRef.current.setCategory(roomCode!, selectedCategory, token!),
                `setCategory ${selectedCategory}`,
              )
            }
          >
            {selectedAlreadyApplied ? "Category Applied" : "Set Category"}
          </button>
          <button data-testid="start-game-btn" disabled={!isAdmin || !token || !roomCode} onClick={() => runAdminAction(() => clientRef.current.startGame(roomCode!, token!), "startGame")}>Start</button>
        </div>
        </section>
      )}

      {showTurnBoard && (
        <section className="card compact-card turn-board-compact" data-testid="turn-board">
        <div className="turn-board-line">
          <p data-testid="turn-board-current">Turn: <strong>{turnPlayerId}</strong></p>
          <div className="row">
            <button
              data-testid="next-turn-btn"
              disabled={!isAdmin || !token || !roomCode}
              onClick={() => runAdminAction(() => clientRef.current.nextTurn(roomCode!, token!), "nextTurn")}
            >
              Next Turn
            </button>
            <button
              data-testid="reveal-result-btn"
              disabled={!isAdmin || !token || !roomCode}
              onClick={() => runAdminAction(() => clientRef.current.revealResult(roomCode!, token!), "revealResult")}
            >
              Reveal Result
            </button>
          </div>
        </div>
        <ul className="turn-list turn-list-inline" data-testid="turn-list">
          {connectedPlayers.map((p) => (
            <li
              key={p.id}
              className={`turn-row ${p.id === turnPlayerId ? "turn-row-active" : ""}`}
              data-testid={`turn-player-${p.id}`}
            >
              <span>{p.nickname}</span>{" "}
              <code>{p.id}</code>
            </li>
          ))}
        </ul>
        </section>
      )}

      {showSuspicionsPanel && (
        <section className="card compact-card" data-testid="suspicions-panel">
        <h2>Suspicions</h2>
        <ul className="player-list" data-testid="suspicion-list">
          {connectedPlayers.map((p) => {
            const suspectedId = suspicionByPlayerId.get(p.id);
            const suspected = suspectedId ? playerById.get(suspectedId) : null;
            return (
              <li key={p.id} className="player-row player-row-compact" data-testid={`suspicion-${p.id}`}>
                <span className="player-name">{p.nickname}</span>
                <span>
                  suspects:{" "}
                  <strong>{suspected ? `${suspected.nickname} (${suspected.id})` : "-"}</strong>
                </span>
              </li>
            );
          })}
        </ul>
        </section>
      )}

      {showChatPanel && (
        <section className="card chat-panel" data-testid="chat-panel">
        <h2>Round Chat</h2>
        <p data-testid="chat-turn-note">
          {canChatThisTurn ? "Your turn: you can chat." : `Waiting for turn (${turnPlayerId})`}
        </p>
        <ul className="chat-list" data-testid="chat-list">
          {chatMessages.length === 0 && (
            <li className="player-empty" data-testid="chat-empty">No messages yet.</li>
          )}
          {chatMessages.map((msg) => (
            <li key={msg.id} className="chat-row" data-testid={`chat-msg-${msg.id}`}>
              <span className="chat-author">{msg.senderNickname}</span>
              <span className="chat-text">{msg.text}</span>
            </li>
          ))}
        </ul>
        <div className="row">
          <input
            data-testid="chat-input"
            value={chatInput}
            onChange={(e) => setChatInput(e.target.value)}
            placeholder={canChatThisTurn ? "ask a quick clue question" : "chat is locked until your turn"}
            disabled={!canChatThisTurn}
          />
          <button data-testid="chat-send-btn" disabled={!token || !roomCode || !canChatThisTurn} onClick={handleSendChat}>Send</button>
        </div>
        </section>
      )}

      {showRoleCard && (
        <section className="card compact-card role-card-slim" data-testid="role-card">
        <h2>Role Card</h2>
        <button
          data-testid="role-toggle-btn"
          onClick={() => setRoleRevealed((v) => !v)}
          disabled={!token || !roomCode}
        >
          {roleRevealed ? "Hide Role" : "Reveal Role"}
        </button>
        {!roleRevealed && (
          <div data-testid="role-hidden">
            <p>Role hidden</p>
            <pre aria-label="blocked-sprite">[####]</pre>
          </div>
        )}
        {roleRevealed && roleView && (
          <div data-testid="role-revealed">
            <p data-testid="role-kind" className={`role-kind ${roleView.gameRole === "IMPOSTER" ? "role-imposter" : "role-crew"}`}>
              {roleView.gameRole}
            </p>
            {roleView.gameRole === "CREW" ? (
              <p data-testid="role-topic">Topic ID: {roleView.topicId ?? "(none)"}</p>
            ) : (
              <p data-testid="role-imposter-msg">You are the imposter.</p>
            )}
          </div>
        )}
        </section>
      )}

      {showGamePanel && (
        <section className="card compact-card game-compact" data-testid="game-panel">
        <h2>Guess Imposter</h2>
        <div className="row">
          <input
            data-testid="guess-player-input"
            value={guessPlayerId}
            onChange={(e) => setGuessPlayerId(e.target.value)}
            placeholder="suspect player id (p2)"
          />
          <button data-testid="guess-btn" disabled={!token || !roomCode} onClick={handleGuessImposter}>Guess Imposter</button>
        </div>
        </section>
      )}

      {showResultPanel && (
        <section className="card" data-testid="result-panel">
        <h2>Result</h2>
        <p>Winner: {snapshot?.result?.winner ?? "-"}</p>
        <p>Guessed: {snapshot?.result?.guessedPlayerId ?? "-"}</p>
        <p>Imposter: {snapshot?.result?.imposterPlayerId ?? "-"}</p>
        <div className="row">
          <button
            data-testid="restart-game-btn"
            disabled={!isAdmin || !token || !roomCode}
            onClick={() => runAdminAction(() => clientRef.current.restartGame(roomCode!, token!), "restartGame")}
          >
            Restart
          </button>
          <button
            data-testid="end-game-btn"
            disabled={!isAdmin || !token || !roomCode}
            onClick={() => runAdminAction(() => clientRef.current.endGame(roomCode!, token!), "endGame")}
          >
            End Game
          </button>
        </div>
        </section>
      )}

      <footer className="status" data-testid="status-line">
        <span data-testid="status-text" className={`alert ${statusTone === "success" ? "alert-success" : "alert-info"}`}>
          {status}
        </span>
        {error && <span className="error">{error}</span>}
      </footer>
    </main>
  );
}
