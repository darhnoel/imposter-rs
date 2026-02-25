# Protocol: JSON over WebSocket

Endpoint:
- `ws://<host>/ws`

## Envelope

Client -> server:

```json
{
  "id": "1",
  "op": "createRoom",
  "payload": { "code": "ABCD", "nickname": "Host" },
  "token": null
}
```

Success response:

```json
{
  "id": "1",
  "type": "response",
  "ok": true,
  "data": { "...": "..." }
}
```

Error response:

```json
{
  "id": "1",
  "type": "response",
  "ok": false,
  "error": { "code": "Forbidden", "message": "caller is not authorized" }
}
```

Push event:

```json
{
  "type": "event",
  "event": "gameUpdated",
  "code": "ABCD",
  "snapshot": { "...": "..." }
}
```

## Supported ops

Queries:
- `categories` payload: `{}`
- `gameSnapshot` payload: `{ "roomCode": "ABCD" }`
- `myRole` payload: `{ "roomCode": "ABCD" }` (`token` required)

Mutations:
- `createRoom` payload: `{ "code": "ABCD", "nickname": "Host" }`
  - response data: `{ "room": RoomView, "token": "<sessionToken>" }`
- `joinRoom` payload: `{ "code": "ABCD", "nickname": "Alice" }`
  - response data: `{ "room": RoomView, "token": "<sessionToken>" }`
- `leaveRoom` payload: `{ "code": "ABCD" }` (`token` required)
- `setCategory` payload: `{ "code": "ABCD", "category": "Wildlands" }` (`token` required)
- `startGame` payload: `{ "code": "ABCD" }` (`token` required)
- `nextTurn` payload: `{ "code": "ABCD" }` (`token` required)
- `guessImposter` payload: `{ "code": "ABCD", "guessedPlayerId": "p2" }` (`token` required)
- `restartGame` payload: `{ "code": "ABCD" }` (`token` required)

## Broadcast points

`gameUpdated` is emitted after successful:
- `joinRoom`
- `leaveRoom`
- `setCategory`
- `startGame`
- `nextTurn`
- `guessImposter`
- `restartGame`

Server behavior on disconnect:
- If a websocket closes and it had an active room session, server marks that player disconnected and emits `gameUpdated`.
