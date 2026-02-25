from __future__ import annotations

import time

from playwright.sync_api import Page

from helpers.markql_assert import (
    assert_any_text_contains,
    assert_row_count_at_least,
    count_rows,
    text_values,
)

PLAYER_ROWS_QUERY = "SELECT li FROM document WHERE attributes.data-testid ~ '^player-'"
PLAYER_TEXT_QUERY = "SELECT TEXT(li) FROM document WHERE attributes.data-testid ~ '^player-'"


def wait_for_player_rows(page: Page, minimum: int, timeout_s: float = 5.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        if count_rows(page.content(), PLAYER_ROWS_QUERY) >= minimum:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for >= {minimum} player rows")


def wait_for_status_contains(page: Page, needle: str, timeout_s: float = 5.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        status = page.get_by_test_id("status-line").inner_text()
        if needle in status:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for status containing: {needle}")


def wait_for_connected(page: Page) -> None:
    start = time.time()
    while (time.time() - start) < 10.0:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if "WS: connected" in banner:
            return
        time.sleep(0.1)
    raise AssertionError("Timed out waiting for ws connected")


def wait_for_phase(page: Page, phase: str, timeout_s: float = 8.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if f"Phase: {phase}" in banner:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for phase {phase}")


def wait_for_phase_or_fail(page: Page, phase: str, fail_needle: str, timeout_s: float = 8.0) -> None:
    start = time.time()
    last_status = ""
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if f"Phase: {phase}" in banner:
            return
        last_status = page.get_by_test_id("status-line").inner_text()
        if fail_needle in last_status:
            raise AssertionError(last_status)
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for phase {phase}. Last status: {last_status}")


def wait_for_success_or_fail(page: Page, success_needle: str, fail_needle: str, timeout_s: float = 5.0) -> None:
    start = time.time()
    last_status = ""
    while (time.time() - start) < timeout_s:
        last_status = page.get_by_test_id("status-line").inner_text()
        if success_needle in last_status:
            return
        if fail_needle in last_status:
            raise AssertionError(last_status)
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for {success_needle}. Last status: {last_status}")


def wait_for_room_or_fail(page: Page, code: str, fail_needle: str, timeout_s: float = 6.0) -> None:
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


def create_room(page: Page, code: str = "ABCD", nickname: str = "Host") -> None:
    wait_for_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill(nickname)
    page.get_by_test_id("create-code-input").fill(code)
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, code, "createRoom failed")


def test_phase_visibility_and_role_default_revealed_contract(page: Page, browser) -> None:
    create_room(page, code="ROLE1", nickname="Host")
    html = page.content()

    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'lobby-panel'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'admin-controls'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'players-panel'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'game-panel'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'turn-board'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'chat-panel'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'role-card'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'result-panel'") == 0

    joiners = []
    for nickname in ("Alice", "Bob"):
        context = browser.new_context()
        other_page = context.new_page()
        other_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
        wait_for_connected(other_page)
        other_page.get_by_test_id("mode-join-btn").click()
        other_page.get_by_test_id("join-nickname-input").fill(nickname)
        other_page.get_by_test_id("join-public-ROLE1").click()
        wait_for_room_or_fail(other_page, "ROLE1", "joinRoom failed")
        joiners.append((context, other_page))

    page.get_by_test_id("set-category-btn").click()
    page.get_by_test_id("start-game-btn").click()
    wait_for_phase_or_fail(page, "IN_PROGRESS", "startGame failed")
    html = page.content()

    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'lobby-panel'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'admin-controls'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'game-panel'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'players-panel'") == 0
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'turn-board'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'chat-panel'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'role-card'") == 1
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'result-panel'") == 0

    page.get_by_test_id("toggle-players-btn").click()
    html = page.content()
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'players-panel'") == 1

    assert_row_count_at_least(
        html,
        "SELECT div FROM document WHERE attributes.data-testid = 'role-revealed'",
        1,
    )
    assert_row_count_at_least(
        html,
        "SELECT p FROM document WHERE parent.attributes.data-testid = 'role-revealed'",
        1,
    )
    assert (
        count_rows(
            html,
            "SELECT div FROM document WHERE attributes.data-testid = 'role-hidden'",
        )
        == 0
    )
    assert (
        count_rows(
            html,
            "SELECT TEXT(button) FROM document WHERE attributes.data-testid = 'role-toggle-btn'",
        )
        == 1
    )
    assert_any_text_contains(
        html,
        "SELECT TEXT(button) FROM document WHERE attributes.data-testid = 'role-toggle-btn'",
        "Hide Role",
    )

    for context, other_page in joiners:
        other_page.close()
        context.close()


def test_connected_player_list_contract(page: Page, browser) -> None:
    create_room(page, code="ROOM2", nickname="Admin")

    other_context = browser.new_context()
    other_page = other_context.new_page()
    other_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(other_page)
    other_page.get_by_test_id("mode-join-btn").click()
    other_page.get_by_test_id("join-nickname-input").fill("Alice")
    other_page.get_by_test_id("join-public-ROOM2").click()
    wait_for_room_or_fail(other_page, "ROOM2", "joinRoom failed")

    wait_for_player_rows(page, 2)
    html = page.content()
    assert_row_count_at_least(html, PLAYER_ROWS_QUERY, 2)
    assert_any_text_contains(html, PLAYER_TEXT_QUERY, "Admin")
    assert_any_text_contains(html, PLAYER_TEXT_QUERY, "ADMIN")
    assert_any_text_contains(html, PLAYER_TEXT_QUERY, "Alice")

    other_page.close()
    other_context.close()

    for _ in range(50):
        html = page.content()
        if count_rows(html, PLAYER_ROWS_QUERY) == 1:
            break
        time.sleep(0.1)

    html = page.content()
    assert count_rows(html, PLAYER_ROWS_QUERY) == 1
    player_texts = text_values(html, PLAYER_TEXT_QUERY)
    assert len(player_texts) == 1
    assert "Admin" in player_texts[0]
    assert "Alice" not in player_texts[0]
