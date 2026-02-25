from __future__ import annotations

import time

from playwright.sync_api import Page

from helpers.markql_assert import assert_any_text_contains, assert_row_count_at_least


def wait_for_status_contains(page: Page, needle: str, timeout_s: float = 10.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        status = page.get_by_test_id("status-line").inner_text()
        if needle in status:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for status containing: {needle}")


def wait_for_ws_connected(page: Page, timeout_s: float = 10.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if "WS: connected" in banner:
            return
        time.sleep(0.1)
    raise AssertionError("Timed out waiting for websocket connected")


def wait_for_room_or_fail(page: Page, code: str, fail_needle: str, timeout_s: float = 8.0) -> None:
    start = time.time()
    last_status = ""
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if f"Room: {code}" in banner:
            return
        last_status = page.get_by_test_id("status-line").inner_text()
        if fail_needle in last_status:
            raise AssertionError(last_status)
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for room {code}. Last status: {last_status}")


def test_admin_controls_exist_with_contract(page: Page) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("PERM")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "PERM", "createRoom failed")

    html = page.content()
    assert_row_count_at_least(
        html,
        "SELECT button FROM document WHERE attributes.data-testid = 'set-category-btn'",
        1,
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(button) FROM document WHERE attributes.data-testid = 'set-category-btn'",
        "Set Category",
    )
    assert_row_count_at_least(
        html,
        "SELECT button FROM document WHERE attributes.data-testid = 'start-game-btn'",
        1,
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(button) FROM document WHERE attributes.data-testid = 'start-game-btn'",
        "Start",
    )
