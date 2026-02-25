from __future__ import annotations

import time

from playwright.sync_api import Page

from helpers.markql_assert import assert_any_text_contains, count_rows


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


def wait_for_room_row(page: Page, code: str, timeout_s: float = 8.0) -> None:
    start = time.time()
    query = f"SELECT li FROM document WHERE attributes.data-testid = 'public-room-{code}'"
    while (time.time() - start) < timeout_s:
        if count_rows(page.content(), query) == 1:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for listed public room {code}")


def test_public_room_is_listed_and_joinable(page: Page, browser) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("HostPub")
    page.get_by_test_id("create-code-input").fill("PUB1")
    page.get_by_test_id("create-public-checkbox").check()
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "PUB1", "createRoom failed")

    join_context = browser.new_context()
    join_page = join_context.new_page()
    join_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_ws_connected(join_page)
    join_page.get_by_test_id("mode-join-btn").click()
    wait_for_room_row(join_page, "PUB1")

    html = join_page.content()
    assert_any_text_contains(
        html,
        "SELECT TEXT(li) FROM document WHERE attributes.data-testid = 'public-room-PUB1'",
        "HostPub",
    )
    join_page.get_by_test_id("join-nickname-input").fill("AlicePub")
    join_page.get_by_test_id("join-public-PUB1").click()
    wait_for_room_or_fail(join_page, "PUB1", "joinRoom failed")

    join_page.close()
    join_context.close()


def test_private_room_is_not_listed(page: Page, browser) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("HostPriv")
    page.get_by_test_id("create-code-input").fill("PRIV1")
    page.get_by_test_id("create-public-checkbox").uncheck()
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "PRIV1", "createRoom failed")

    join_context = browser.new_context()
    join_page = join_context.new_page()
    join_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_ws_connected(join_page)
    join_page.get_by_test_id("mode-join-btn").click()
    time.sleep(0.4)

    assert (
        count_rows(
            join_page.content(),
            "SELECT li FROM document WHERE attributes.data-testid = 'public-room-PRIV1'",
        )
        == 0
    )

    join_page.close()
    join_context.close()
