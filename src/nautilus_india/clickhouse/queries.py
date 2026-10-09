"""Parameterized ClickHouse queries.

Bounds are UNIX nanoseconds. ``start_ns`` is inclusive and ``end_ns`` is
exclusive, matching a Nautilus request window. A positive ``limit`` keeps the
newest rows inside that window and the result is returned in ascending time.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class BoundQuery:
    sql: str
    parameters: dict[str, object]


def select_instruments(
    *,
    instrument_ids: list[str] | None = None,
    venues: list[str] | None = None,
    products: list[str] | None = None,
    underlyings: list[str] | None = None,
) -> BoundQuery:
    clauses = ["1"]
    parameters: dict[str, object] = {}
    _equals_any(clauses, parameters, "instrument_id", instrument_ids)
    _equals_any(clauses, parameters, "venue", venues)
    _equals_any(clauses, parameters, "product", products)
    _equals_any(clauses, parameters, "underlying", underlyings)
    where = " AND ".join(clauses)
    sql = f"""
        SELECT
            instrument_id,
            venue,
            raw_symbol,
            product,
            asset_class,
            underlying,
            expiry,
            strike,
            option_kind,
            currency,
            price_precision,
            price_increment,
            lot_size,
            multiplier,
            isin,
            series,
            segment,
            activation_ns,
            expiration_ns
        FROM instruments FINAL
        WHERE {where}
        ORDER BY venue, instrument_id
    """
    return BoundQuery(sql=sql, parameters=parameters)


def select_bars(
    *,
    instrument_id: str,
    bar_spec: str,
    start_ns: int | None = None,
    end_ns: int | None = None,
    limit: int | None = None,
) -> BoundQuery:
    return _select_time_series(
        table="bars",
        columns=(
            "instrument_id",
            "bar_spec",
            "ts_event",
            "open",
            "high",
            "low",
            "close",
            "volume",
            "open_interest",
        ),
        instrument_id=instrument_id,
        extra=[("bar_spec", bar_spec)],
        start_ns=start_ns,
        end_ns=end_ns,
        limit=limit,
    )


def select_trades(
    *,
    instrument_id: str,
    start_ns: int | None = None,
    end_ns: int | None = None,
    limit: int | None = None,
) -> BoundQuery:
    return _select_time_series(
        table="trades",
        columns=(
            "instrument_id",
            "ts_event",
            "trade_id",
            "price",
            "size",
            "aggressor_side",
        ),
        instrument_id=instrument_id,
        start_ns=start_ns,
        end_ns=end_ns,
        limit=limit,
    )


def select_quotes(
    *,
    instrument_id: str,
    start_ns: int | None = None,
    end_ns: int | None = None,
    limit: int | None = None,
) -> BoundQuery:
    return _select_time_series(
        table="quotes",
        columns=(
            "instrument_id",
            "ts_event",
            "bid_price",
            "ask_price",
            "bid_size",
            "ask_size",
        ),
        instrument_id=instrument_id,
        start_ns=start_ns,
        end_ns=end_ns,
        limit=limit,
    )


def select_holidays(*, venue: str) -> BoundQuery:
    return BoundQuery(
        sql="""
            SELECT session_date
            FROM trading_holidays FINAL
            WHERE venue = {venue:String}
            ORDER BY session_date
        """,
        parameters={"venue": venue},
    )


def _select_time_series(
    *,
    table: str,
    columns: tuple[str, ...],
    instrument_id: str,
    extra: list[tuple[str, str]] | None = None,
    start_ns: int | None = None,
    end_ns: int | None = None,
    limit: int | None = None,
) -> BoundQuery:
    clauses = ["instrument_id = {instrument_id:String}"]
    parameters: dict[str, object] = {"instrument_id": instrument_id}
    for name, value in extra or []:
        clauses.append(f"{name} = {{{name}:String}}")
        parameters[name] = value
    if start_ns is not None:
        clauses.append("ts_event >= {start_ns:Int64}")
        parameters["start_ns"] = start_ns
    if end_ns is not None:
        clauses.append("ts_event < {end_ns:Int64}")
        parameters["end_ns"] = end_ns
    where = " AND ".join(clauses)
    selected = ", ".join(columns)
    if limit is not None and limit > 0:
        sql = f"""
            SELECT {selected}
            FROM (
                SELECT {selected}
                FROM {table} FINAL
                WHERE {where}
                ORDER BY ts_event DESC
                LIMIT {{limit:UInt32}}
            )
            ORDER BY ts_event
        """
        parameters["limit"] = limit
    else:
        sql = f"""
            SELECT {selected}
            FROM {table} FINAL
            WHERE {where}
            ORDER BY ts_event
        """
    return BoundQuery(sql=sql, parameters=parameters)


def _equals_any(
    clauses: list[str],
    parameters: dict[str, object],
    column: str,
    values: list[str] | None,
) -> None:
    if not values:
        return
    clauses.append(f"{column} IN {{{column}:Array(String)}}")
    parameters[column] = list(values)
