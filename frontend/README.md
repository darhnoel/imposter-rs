# imposter-rs frontend

React + TypeScript web client for `server_ws`.

## Run

```bash
cd frontend
npm install
npm run dev
```

Default WS endpoint:
- `ws://127.0.0.1:4000/ws`

Override with:

```bash
VITE_WS_URL=ws://127.0.0.1:4100/ws npm run dev
```

## Notes
- Protocol is documented in `../docs/frontend-contract.md`.
- UI actions use Playwright locators; DOM structure contracts are asserted with `pyxsql` in Python E2E tests.
