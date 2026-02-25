from __future__ import annotations

import os
import socket
import subprocess
import time
from pathlib import Path

import pytest
from playwright.sync_api import Browser, BrowserContext, Page, sync_playwright


REPO_ROOT = Path(__file__).resolve().parents[3]
FRONTEND_DIR = REPO_ROOT / "frontend"



def wait_for_port(host: str, port: int, timeout: float = 30.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.settimeout(0.5)
            if sock.connect_ex((host, port)) == 0:
                return
        time.sleep(0.2)
    raise RuntimeError(f"Timed out waiting for {host}:{port}")


@pytest.fixture(scope="session")
def backend_server() -> subprocess.Popen[str]:
    env = os.environ.copy()
    env["IMPOSTER_WS_BIND"] = "127.0.0.1:4000"
    proc = subprocess.Popen(
        ["cargo", "run", "-p", "server_ws"],
        cwd=REPO_ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    wait_for_port("127.0.0.1", 4000)
    yield proc
    proc.terminate()
    proc.wait(timeout=10)


@pytest.fixture(scope="session")
def frontend_server(backend_server: subprocess.Popen[str]) -> subprocess.Popen[str]:
    env = os.environ.copy()
    env["VITE_WS_URL"] = "ws://127.0.0.1:4000/ws"
    proc = subprocess.Popen(
        ["npm", "run", "dev", "--", "--host", "127.0.0.1", "--port", "5173"],
        cwd=FRONTEND_DIR,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    wait_for_port("127.0.0.1", 5173)
    yield proc
    proc.terminate()
    proc.wait(timeout=10)


@pytest.fixture(scope="session")
def browser(frontend_server: subprocess.Popen[str]) -> Browser:
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        yield browser
        browser.close()


@pytest.fixture()
def page(browser: Browser) -> Page:
    context: BrowserContext = browser.new_context()
    page = context.new_page()
    page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    yield page
    context.close()
