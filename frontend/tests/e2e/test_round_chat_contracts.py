from __future__ import annotations

import time

from playwright.sync_api import Browser, Page

from helpers.markql_assert import assert_any_text_contains, count_rows


def wait_for_connected(page: Page, timeout_s: float = 10.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if "WS: connected" in banner:
            return
        time.sleep(0.1)
    raise AssertionError("Timed out waiting for ws connected")


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


def wait_for_phase(page: Page, phase: str, timeout_s: float = 10.0) -> None:
    start = time.time()
    while (time.time() - start) < timeout_s:
        banner = page.get_by_test_id("connection-banner").inner_text()
        if f"Phase: {phase}" in banner:
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for phase {phase}")


def wait_for_turn_to_change(page: Page, initial: str, timeout_s: float = 6.0) -> str:
    start = time.time()
    while (time.time() - start) < timeout_s:
        current = page.get_by_test_id("turn-board-current").inner_text()
        if current != initial:
            return current
        time.sleep(0.1)
    raise AssertionError(f"Turn text did not change from {initial}")


def test_turn_board_visible_and_advances_for_admin(page: Page, browser: Browser) -> None:
    wait_for_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("TURN1")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "TURN1", "createRoom failed")

    join_ctx = browser.new_context()
    join_page = join_ctx.new_page()
    join_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page)
    join_page.get_by_test_id("mode-join-btn").click()
    join_page.get_by_test_id("join-nickname-input").fill("Alice")
    join_page.get_by_test_id("join-public-TURN1").click()
    wait_for_room_or_fail(join_page, "TURN1", "joinRoom failed")

    join_ctx_2 = browser.new_context()
    join_page_2 = join_ctx_2.new_page()
    join_page_2.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page_2)
    join_page_2.get_by_test_id("mode-join-btn").click()
    join_page_2.get_by_test_id("join-nickname-input").fill("Bob")
    join_page_2.get_by_test_id("join-public-TURN1").click()
    wait_for_room_or_fail(join_page_2, "TURN1", "joinRoom failed")

    page.get_by_test_id("set-category-btn").click()
    page.get_by_test_id("start-game-btn").click()
    wait_for_phase(page, "IN_PROGRESS")

    html = page.content()
    assert count_rows(html, "SELECT section FROM document WHERE attributes.data-testid = 'turn-board'") == 1
    assert_any_text_contains(
        html,
        "SELECT TEXT(button) FROM document WHERE attributes.data-testid = 'next-turn-btn'",
        "Next Turn",
    )

    before = page.get_by_test_id("turn-board-current").inner_text()
    page.get_by_test_id("next-turn-btn").click()
    after = wait_for_turn_to_change(page, before)
    assert before != after

    join_page.close()
    join_ctx.close()
    join_page_2.close()
    join_ctx_2.close()


def test_round_chat_displays_sender_and_message_content(page: Page, browser: Browser) -> None:
    wait_for_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("CHAT1")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "CHAT1", "createRoom failed")

    join_ctx = browser.new_context()
    join_page = join_ctx.new_page()
    join_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page)
    join_page.get_by_test_id("mode-join-btn").click()
    join_page.get_by_test_id("join-nickname-input").fill("Alice")
    join_page.get_by_test_id("join-public-CHAT1").click()
    wait_for_room_or_fail(join_page, "CHAT1", "joinRoom failed")

    join_ctx_2 = browser.new_context()
    join_page_2 = join_ctx_2.new_page()
    join_page_2.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page_2)
    join_page_2.get_by_test_id("mode-join-btn").click()
    join_page_2.get_by_test_id("join-nickname-input").fill("Bob")
    join_page_2.get_by_test_id("join-public-CHAT1").click()
    wait_for_room_or_fail(join_page_2, "CHAT1", "joinRoom failed")

    page.get_by_test_id("set-category-btn").click()
    page.get_by_test_id("start-game-btn").click()
    wait_for_phase(page, "IN_PROGRESS")
    wait_for_phase(join_page, "IN_PROGRESS")
    page.get_by_test_id("next-turn-btn").click()

    join_page.get_by_test_id("chat-input").fill("What is your clue about?")
    join_page.get_by_test_id("chat-send-btn").click()

    start = time.time()
    while (time.time() - start) < 6.0:
        html = page.content()
        values_query = "SELECT TEXT(li) FROM document WHERE attributes.data-testid ~ '^chat-msg-'"
        if count_rows(html, "SELECT li FROM document WHERE attributes.data-testid ~ '^chat-msg-'") > 0:
            assert_any_text_contains(html, values_query, "Alice")
            assert_any_text_contains(html, values_query, "What is your clue about?")
            break
        time.sleep(0.1)
    else:
        raise AssertionError("Timed out waiting for chat message on admin page")

    join_page.close()
    join_ctx.close()
    join_page_2.close()
    join_ctx_2.close()


def test_suspicion_board_updates_and_only_admin_can_reveal(page: Page, browser: Browser) -> None:
    wait_for_connected(page)
    page.get_by_test_id("mode-admin-btn").click()
    page.get_by_test_id("create-nickname-input").fill("Host")
    page.get_by_test_id("create-code-input").fill("SUSP1")
    page.get_by_test_id("create-room-btn").click()
    wait_for_room_or_fail(page, "SUSP1", "createRoom failed")

    join_ctx = browser.new_context()
    join_page = join_ctx.new_page()
    join_page.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page)
    join_page.get_by_test_id("mode-join-btn").click()
    join_page.get_by_test_id("join-nickname-input").fill("Alice")
    join_page.get_by_test_id("join-public-SUSP1").click()
    wait_for_room_or_fail(join_page, "SUSP1", "joinRoom failed")

    join_ctx_2 = browser.new_context()
    join_page_2 = join_ctx_2.new_page()
    join_page_2.goto("http://127.0.0.1:5173", wait_until="networkidle")
    wait_for_connected(join_page_2)
    join_page_2.get_by_test_id("mode-join-btn").click()
    join_page_2.get_by_test_id("join-nickname-input").fill("Bob")
    join_page_2.get_by_test_id("join-public-SUSP1").click()
    wait_for_room_or_fail(join_page_2, "SUSP1", "joinRoom failed")

    page.get_by_test_id("set-category-btn").click()
    page.get_by_test_id("start-game-btn").click()
    wait_for_phase(page, "IN_PROGRESS")
    wait_for_phase(join_page, "IN_PROGRESS")

    join_page.get_by_test_id("guess-player-input").fill("p3")
    join_page.get_by_test_id("guess-btn").click()

    start = time.time()
    while (time.time() - start) < 6.0:
        html = page.content()
        query = "SELECT TEXT(li) FROM document WHERE attributes.data-testid = 'suspicion-p2'"
        if count_rows(html, "SELECT li FROM document WHERE attributes.data-testid = 'suspicion-p2'") == 1:
            assert_any_text_contains(html, query, "Bob (p3)")
            break
        time.sleep(0.1)
    else:
        raise AssertionError("Timed out waiting for suspicion board update")

    assert "Phase: IN_PROGRESS" in page.get_by_test_id("connection-banner").inner_text()
    assert join_page.get_by_test_id("reveal-result-btn").is_disabled()

    page.get_by_test_id("reveal-result-btn").click()
    wait_for_phase(page, "COMPLETED")
    wait_for_phase(join_page, "COMPLETED")

    html = page.content()
    assert_any_text_contains(
        html,
        "SELECT TEXT(p) FROM document WHERE parent.attributes.data-testid = 'result-panel'",
        "Imposter:",
    )

    join_page.close()
    join_ctx.close()
    join_page_2.close()
    join_ctx_2.close()
