# Architecture

## Layers

1. `crates/domain`
- Pure state machine (`apply(state, command)`).
- Owns invariants, stable errors, deterministic RNG trait.
- Owns topic catalog (`Animals`, `Countries`, `Foods`).

2. `crates/service`
- Contract-level API aligned with future GraphQL operations.
- Uses `RoomStore` abstraction and per-room locking.
- Handles token hashing and authorization checks.
- Emits `gameUpdated` snapshots and `chatMessage` events via broadcast channels.
- Enforces turn-gated chat and auto-advances turn after successful chat send.

3. `crates/server_ws`
- Thin transport adapter for JSON-over-WebSocket.
- Parses envelope, dispatches to service, formats responses/events.
- Maintains one subscription stream per connected client room.

4. `frontend` (React + TypeScript)
- WebSocket client using the same operation envelope contract.
- Renders Lobby/Game/Result panels from `GameSnapshot.phase`.
- Uses Playwright + pyxsql (MarkQL) for DOM contract tests.

## Store and Concurrency

- `InMemoryRoomStore` keeps `HashMap<roomCode, RoomEntry>`.
- Each `RoomEntry` has:
  - `Mutex<RoomState>`
  - `Mutex<HashMap<TokenHash, PlayerId>>`
  - `broadcast::Sender<GameSnapshot>`
- All mutations serialize via per-room mutex lock.

## Migration note to GraphQL

GraphQL resolvers can be added later as thin wrappers over `GameService` functions:
- query resolvers -> `categories`, `game_snapshot`, `my_role`
- mutation resolvers -> `create_room`, `join_room`, `set_category`, etc.
- subscription resolver -> `subscribe_game_updated`

This keeps migration mostly mechanical while preserving tested domain/service behavior.
