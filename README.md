# imposter-rs

Rust workspace for an Imposter game MVP.

## Workspace

- `crates/domain`: pure game rules and state transitions
- `crates/service`: application/service layer API
- `crates/server_ws`: JSON-over-WebSocket server transport
- `frontend`: React + TypeScript client

## Prerequisites

- Rust toolchain (`cargo`)
- Node.js + npm (for frontend)
- Python 3 (optional, for frontend E2E tests)

## Run Backend

```bash
cargo run -p server_ws
```

Optional bind override:

```bash
IMPOSTER_WS_BIND=127.0.0.1:4100 cargo run -p server_ws
```

## Run Frontend

```bash
cd frontend
npm install
npm run dev
```

Optional WS endpoint override:

```bash
VITE_WS_URL=ws://127.0.0.1:4100/ws npm run dev
```

## Example Room Flow

1. Create room with code `ABCD`
2. Join room from other clients
3. Set category (`Animals`, `Countries`, or `Foods`)
4. Start game
5. Play turns, submit guesses, reveal result

## Test

Run all Rust tests:

```bash
cargo test
```

Run frontend E2E (`Playwright + pyxsql`):

```bash
cd frontend/tests/e2e
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
playwright install chromium
pytest
```

## Docs

- `docs/architecture.md`
- `docs/protocol-ws.md`
- `docs/testing.md`
- `docs/frontend-contract.md`
- `docs/ux.md`
