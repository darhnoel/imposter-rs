# Testing

## Run all tests

```bash
cargo test -q
```

## Frontend E2E (Playwright + pyxsql/MarkQL)

```bash
cd frontend/tests/e2e
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
.venv/bin/playwright install chromium
.venv/bin/pytest -q
```

## Run domain tests only

```bash
cargo test -p domain -q
```

## Run server_ws tests only

```bash
cargo test -p server_ws -q
```

## What is covered

Domain:
- min/max player constraints
- deterministic imposter/topic selection
- turn wrap-around
- suspicion submission without ending round
- reveal deciding winner from most-suspected player
- restart behavior
- private role visibility rules

Service/server_ws:
- admin gating for restricted operations
- token/session validation
- `gameUpdated` broadcast emission
- `chatMessage` emission
- turn auto-advance after successful `sendChat`
- end-to-end flow:
  - `createRoom -> joinRoom -> setCategory -> startGame -> turn/chat/suspicion/reveal -> restart/end`

Frontend E2E:
- Playwright drives browser actions (`create/join/start/chat/guess/reveal/restart` flows).
- `pyxsql` (MarkQL, imported as `xsql`) validates DOM contracts on `page.content()` snapshots.
- Contract checks include:
  - lobby/public room join flows
  - role card contracts
  - phase-based panel visibility
  - admin permission boundaries
  - error handling contracts
