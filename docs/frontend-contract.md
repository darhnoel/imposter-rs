# Frontend Contract (Current MVP)

This document freezes the web frontend contract against `server_ws`.

For full rebuild requirements, see `docs/CODEX_REBUILD_GUIDE.md`.

Endpoint:
- `ws://127.0.0.1:4000/ws` (default)

Envelope:

```json
{
  "id": "1",
  "op": "<operationName>",
  "payload": {},
  "token": "<optional session token>"
}
```

Success:

```json
{ "id": "1", "type": "response", "ok": true, "data": {} }
```

Error:

```json
{
  "id": "1",
  "type": "response",
  "ok": false,
  "error": { "code": "...", "message": "..." }
}
```

Push events:

```json
{ "type": "event", "event": "gameUpdated", "code": "ABCD", "snapshot": {} }
```

```json
{ "type": "event", "event": "chatMessage", "code": "ABCD", "message": {} }
```

## Query Operations
- `categories` payload: `{}`
- `listRooms` payload: `{}`
- `gameSnapshot` payload: `{ "roomCode": "ABCD" }`
- `myRole` payload: `{ "roomCode": "ABCD" }` (`token` required)
- `chatHistory` payload: `{ "code": "ABCD" }` (`token` required)

## Mutation Operations
- `createRoom` payload: `{ "code": "ABCD", "nickname": "Host", "public": true|false }`
  - returns: `{ "room": RoomView, "token": "..." }`
- `joinRoom` payload: `{ "code": "ABCD", "nickname": "Alice" }`
  - returns: `{ "room": RoomView, "token": "..." }`
- `leaveRoom` payload: `{ "code": "ABCD" }` (`token` required)
- `setCategory` payload: `{ "code": "ABCD", "category": "Countries" }` (`token` required)
- `startGame` payload: `{ "code": "ABCD" }` (`token` required)
- `nextTurn` payload: `{ "code": "ABCD" }` (`token` required)
- `guessImposter` payload: `{ "code": "ABCD", "guessedPlayerId": "p2" }` (`token` required)
- `revealResult` payload: `{ "code": "ABCD" }` (`token` required)
- `restartGame` payload: `{ "code": "ABCD" }` (`token` required)
- `endGame` payload: `{ "code": "ABCD" }` (`token` required)
- `sendChat` payload: `{ "code": "ABCD", "text": "..." }` (`token` required)

## Broadcast Expectations
`gameUpdated` is expected after successful:
- `createRoom`, `joinRoom`, `leaveRoom`, `setCategory`, `startGame`, `nextTurn`, `guessImposter`, `revealResult`, `restartGame`, `endGame`
- `sendChat` also emits `gameUpdated` because sending chat auto-advances the turn

`chatMessage` is expected after successful:
- `sendChat`

## Frontend State Requirements
- Players card:
  - visible outside `IN_PROGRESS`
  - hidden by default in `IN_PROGRESS`
  - toggleable in `IN_PROGRESS`
- Role card:
  - visible in `IN_PROGRESS`
  - role revealed by default
  - hide/reveal toggle available
- `myRole` should be fetched for `IN_PROGRESS` when role is revealed.
- Only connected players rendered in visible player list.
- Admin controls enabled only for admin session.
