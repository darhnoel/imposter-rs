# Frontend E2E (Playwright + pyxsql)

These tests combine:
- Playwright for browser interactions
- `pyxsql` (MarkQL Python package) for DOM contract assertions over `page.content()`

## Setup

```bash
cd frontend/tests/e2e
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
playwright install chromium
```

## Run

```bash
pytest
```

## Notes

- The test fixture starts backend (`cargo run -p server_ws`) and frontend (`npm run dev`) automatically.
- Ensure `npm install` has been run in `frontend/` first.
- Contract reference: `../../../docs/frontend-contract.md`.
