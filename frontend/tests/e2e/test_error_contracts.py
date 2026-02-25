from __future__ import annotations

import time

from playwright.sync_api import Browser, Page

from helpers.markql_assert import assert_any_text_contains


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


def wait_for_phase(page: Page, phase: str, timeout_s: float = 8.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if f"Phase: {phase}" in banner:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for phase {phase}")


def test_join_requires_nickname_error_is_shown(page: Page) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("JOINE")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "JOINE", "createRoom failed")
    page.get_by_test_id("leave-room-btn").click()

    page.get_by_test_id("mode-join-btn").click()
    page.get_by_test_id("join-public-JOINE").click()
    wait_for_status_contains(page, "joinRoom failed")

    html = page.content()
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "joinRoom failed",
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "nickname is required",
    )


def test_start_game_insufficient_players_error_is_shown(page: Page) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("ERRS")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "ERRS", "createRoom failed")

    page.get_by_test_id("start-game-btn").click()
    wait_for_status_contains(page, "startGame failed")

    html = page.content()
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "startGame failed",
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "not enough players to start",
    )


def test_send_chat_empty_message_error_is_shown(page: Page, browser: Browser) -> None:
    wait_for_ws_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("CHATERR")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "CHATERR", "createRoom failed")

    other_context = browser.new_context()
    other_page = other_context.new_page()
    other_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_ws_connected(other_page)
    other_page.get_by_test_id("mode-join-btn").click()
    other_page.get_by_test_id("join-nickname-input").fill("Alice")
    other_page.get_by_test_id("join-public-CHATERR").click()
    wait_for_room_or_fail(other_page, "CHATERR", "joinRoom failed")

    third_context = browser.new_context()
    third_page = third_context.new_page()
    third_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_ws_connected(third_page)
    third_page.get_by_test_id("mode-join-btn").click()
    third_page.get_by_test_id("join-nickname-input").fill("Bob")
    third_page.get_by_test_id("join-public-CHATERR").click()
    wait_for_room_or_fail(third_page, "CHATERR", "joinRoom failed")

    page.get_by_test_id("set-category-btn").click()
    page.get_by_test_id("start-game-btn").click()
    wait_for_phase(page, "IN_PROGRESS")

    page.get_by_test_id("chat-input").fill("   ")
    page.get_by_test_id("chat-send-btn").click()
    wait_for_status_contains(page, "sendChat failed")

    html = page.content()
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "sendChat failed",
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(span) FROM document WHERE attributes.class = 'error'",
        "message is required",
    )

    other_page.close()
    other_context.close()
    third_page.close()
    third_context.close()
