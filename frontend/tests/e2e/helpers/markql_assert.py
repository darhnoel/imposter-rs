from __future__ import annotations

from typing import Any

import xsql


def query_rows(html: str, query: str) -> list[dict[str, Any]]:
    doc = xsql.load(html)
    result = xsql.execute(query, doc=doc)
    return result.rows


def count_rows(html: str, query: str) -> int:
    return len(query_rows(html, query))


def assert_row_count_at_least(html: str, query: str, minimum: int) -> None:
    actual = count_rows(html, query)
    assert actual >= minimum, f"Expected at least {minimum} rows, got {actual}. Query: {query}"


def text_values(html: str, query: str, field: str = "text") -> list[str]:
    rows = query_rows(html, query)
    values: list[str] = []
    for row in rows:
        value = row.get(field)
        if isinstance(value, str):
            values.append(value)
    return values


def assert_any_text_contains(html: str, query: str, needle: str, field: str = "text") -> None:
    values = text_values(html, query, field=field)
    assert any(needle in value for value in values), (
        f"Expected at least one row to contain {needle!r}. Query: {query}. Values: {values}"
    )
